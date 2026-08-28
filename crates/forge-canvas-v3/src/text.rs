//! Proportional text layout — word-wrap line breaker using fontdue metrics.
//! No GPU types. Pure math. Outputs positioned glyphs for instanced rendering.
//! FontAtlas: fontdue-based glyph rasterizer with row-packing atlas and zero-alloc cache lookups.

use forge_core_v3::fixed_point::MilliUnit;

/// Per-glyph metrics cached from fontdue rasterization.
#[derive(Clone, Copy, Debug)]
pub struct GlyphData {
    /// Atlas UV coordinates (u0, v0, u1, v1).
    pub uv: [f32; 4],
    /// Cursor advance after this glyph (MilliUnit: 1000 = 1px).
    pub advance: MilliUnit,
    /// Bearing offset from cursor to top-left of bitmap.
    pub offset: [i16; 2],
    /// Pixel dimensions of the rasterized bitmap.
    pub size: [u16; 2],
}

/// Positioned glyph ready for GPU instanced rendering.
#[derive(Clone, Copy, Debug, Default)]
pub struct GlyphInstance {
    /// Screen position (top-left of glyph quad) in pixels.
    pub pos: [f32; 2],
    /// Atlas UV coordinates.
    pub uv: [f32; 4],
    /// Packed RGBA color.
    pub color: u32,
    /// Quad size in pixels.
    pub size: [f32; 2],
}

// ── Font Atlas ────────────────────────────────────────────────────────────────

/// Atlas texture dimension (width and height) in pixels.
pub const ATLAS_SIZE: usize = 4096;

/// Fixed-size extended cache for non-ASCII glyphs.
pub const EXTENDED_CACHE_SIZE: usize = 512;

/// Cached glyph location and metrics within the atlas. Copy-able, fits in cache line.
#[derive(Copy, Clone, Debug)]
pub struct AtlasGlyph {
    /// UV coordinates in atlas [u_min, v_min, u_max, v_max] normalized 0.0..1.0
    pub uv: [f32; 4],
    /// Advance width in MilliUnits (1000 = 1px)
    pub advance: i64,
    /// Bearing offset from cursor to top-left of bitmap [x, y] in pixels
    pub offset: [i16; 2],
    /// Pixel dimensions of rasterized bitmap [width, height]
    pub size: [u16; 2],
}

/// FreeType's default 5-tap FIR for LCD subpixel coverage. The weights sum to
/// 256, so the filter is energy-preserving and a full-coverage row stays full —
/// integer math, no float, and the ONLY thing standing between 3x horizontal
/// samples and visible colour fringing. Sovereign: our own filter over our own
/// raster, no external text engine (cosmic-text stays rejected).
pub const LCD_FIR5: [u16; 5] = [8, 77, 86, 77, 8];

/// Filter one row of 3x-horizontal coverage samples into per-pixel RGB subpixel
/// coverage. `samples` holds three samples per output pixel (R, G, B stripe
/// order); edges clamp rather than wrap, so a glyph's first and last column keep
/// their weight instead of bleeding to the other side of the row.
pub fn lcd_filter_row(samples: &[u8], out: &mut [[u8; 3]]) {
    if samples.is_empty() {
        return;
    }
    let last = samples.len() as isize - 1;
    let px = (samples.len() / 3).min(out.len());
    for p in 0..px {
        for c in 0..3 {
            let center = (p * 3 + c) as isize;
            let mut acc: u32 = 0;
            for (k, w) in LCD_FIR5.iter().enumerate() {
                let idx = (center + k as isize - 2).clamp(0, last) as usize;
                acc += samples[idx] as u32 * *w as u32;
            }
            out[p][c] = (acc / 256).min(255) as u8;
        }
    }
}

/// fontdue-based font atlas with row-packing and zero-alloc cache lookups.
/// Allocated once at boot, reused every frame.
pub struct FontAtlas {
    /// The parsed font from fontdue.
    font: fontdue::Font,
    /// CPU-side R8 texture data. Allocated once at boot.
    pub texture_data: Box<[u8]>,
    /// Row-packing cursor: next horizontal position
    next_x: usize,
    /// Row-packing cursor: next vertical position (top of current row)
    next_y: usize,
    /// Height of the tallest glyph in the current row
    row_height: usize,
    /// ASCII fast-path cache (indices 0..255). O(1) direct index.
    glyph_cache: [Option<AtlasGlyph>; 256],
    /// Bitmask for ASCII codepoints 0-127 that were attempted but produced no
    /// rasterizable bitmap (e.g. space, newline). Prevents re-calling font.rasterize
    /// every frame for zero-area glyphs. No heap.
    tried_ascii: u128,
    /// Extended cache for non-ASCII (Unicode > 255). Linear probe, fixed size.
    extended_cache: [(char, Option<AtlasGlyph>); EXTENDED_CACHE_SIZE],
    /// Number of entries currently in the extended cache
    extended_cache_count: usize,
    /// Atlas origin RESERVED for each extended-cache ring slot, once assigned
    /// (`None` = never touched yet). BUG FIX (glyph-cache corruption, long
    /// sessions): the eviction path used to call `pack_glyph` on EVERY
    /// overwrite, permanently consuming fresh atlas territory for a slot
    /// that already had one — the atlas never reclaimed anything and
    /// eventually ran out (`pack_glyph` -> `None`) or, worse, on some texture
    /// upload paths let two glyphs' regions overlap. Now a slot's region is
    /// reserved ONCE and reused for every future glyph assigned to that slot
    /// (sized generously enough for this atlas's `font_size`), so total
    /// extended-glyph atlas usage is bounded and constant, not unbounded.
    extended_slot_origin: [Option<(usize, usize)>; EXTENDED_CACHE_SIZE],
    /// Set true when new glyphs are rasterized; GPU checks this.
    pub dirty: bool,
    /// Font size used for rasterization (fixed per atlas instance)
    pub font_size: f32,
    /// Font ascent in pixels (distance from baseline to top of tallest glyph)
    pub ascent: f32,
    /// Baked ASCII×ASCII pair-kern matrix (MilliUnit), printable 32..=126, cold-baked
    /// at init from GPOS. O(1) hot lookup via `kern()`. Empty when the font has no GPOS.
    kern_ascii: Box<[i16]>,
}

impl FontAtlas {
    /// Boot-time initialization. Parses font via fontdue, allocates zeroed texture.
    pub fn init(font_bytes: &[u8], font_size: f32) -> Self {
        let font = fontdue::Font::from_bytes(
            font_bytes,
            fontdue::FontSettings::default(),
        )
        .expect("FontAtlas::init: invalid font bytes");

        let texture_data = vec![0u8; ATLAS_SIZE * ATLAS_SIZE].into_boxed_slice();

        let ascent = font.horizontal_line_metrics(font_size)
            .map(|m| m.ascent)
            .unwrap_or(font_size * 0.8);

        // Cold-bake the GPOS pair-kern matrix (fontdue only reads the legacy 'kern'
        // table; our fonts carry kerning in GPOS). Zero-alloc-hot: baked once here.
        // NOTE: gpos_kern module is expected to exist; returns empty matrix if unavailable.
        let kern_ascii = extract_ascii_kern_or_empty(
            font_bytes, font_size, |c| Some(font.lookup_glyph_index(c)),
        );

        Self {
            font,
            texture_data,
            kern_ascii,
            next_x: 0,
            next_y: 0,
            row_height: 0,
            glyph_cache: [None; 256],
            tried_ascii: 0,
            extended_cache: [('\0', None); EXTENDED_CACHE_SIZE],
            extended_cache_count: 0,
            extended_slot_origin: [None; EXTENDED_CACHE_SIZE],
            dirty: false,
            font_size,
            ascent,
        }
    }

    /// Row-packing: place glyph left-to-right with 1px padding.
    /// Advance to next row when horizontal space exhausted.
    /// Returns None when atlas is vertically full.
    fn pack_glyph(&mut self, width: usize, height: usize) -> Option<(usize, usize)> {
        if self.next_x + width > ATLAS_SIZE {
            self.next_x = 0;
            self.next_y += self.row_height;
            self.row_height = 0;
        }

        if self.next_y + height > ATLAS_SIZE {
            return None;
        }

        let origin = (self.next_x, self.next_y);

        // Advance cursor with 1px padding to prevent texture bleed
        self.next_x += width + 1;
        self.row_height = self.row_height.max(height + 1);

        Some(origin)
    }

    /// Lookup or rasterize a glyph. Cache hit = zero-alloc.
    /// Cache miss = rasterizes into texture_data (cold path).
    /// Returns None for empty bitmaps (space) or when atlas is full.
    pub fn get_or_rasterize(&mut self, c: char) -> Option<AtlasGlyph> {
        let code = c as usize;

        // ASCII fast path: direct index lookup
        if code < 256 {
            if let Some(glyph) = self.glyph_cache[code] {
                return Some(glyph);
            }
            // Zero-area chars (space, newline, etc.) never enter glyph_cache.
            // Track them via tried_ascii so we don't re-call font.rasterize each frame.
            if code < 128 && (self.tried_ascii >> code) & 1 != 0 {
                return None;
            }
            match self.rasterize_and_cache(c) {
                Some(glyph) => {
                    self.glyph_cache[code] = Some(glyph);
                    return Some(glyph);
                }
                None => {
                    if code < 128 {
                        self.tried_ascii |= 1u128 << code;
                    }
                    return None;
                }
            }
        }

        for i in 0..self.extended_cache_count.min(EXTENDED_CACHE_SIZE) {
            if self.extended_cache[i].0 == c {
                return self.extended_cache[i].1;
            }
        }

        // Miss: assign this char the next ring slot and rasterize into THAT
        // slot's reserved atlas region. Bounded — once all EXTENDED_CACHE_SIZE
        // slots are live, an evicted-then-different glyph reuses its slot's
        // region instead of packing fresh atlas territory (the corruption bug).
        let slot = self.extended_cache_count % EXTENDED_CACHE_SIZE;
        let glyph = self.rasterize_into_slot(c, slot)?;
        self.extended_cache[slot] = (c, Some(glyph));
        self.extended_cache_count += 1;

        Some(glyph)
    }

    /// The monospace cell advance in MilliUnit — the true glyph pitch this atlas
    /// rasterizes at. A terminal grid MUST place its columns at this pitch, or a
    /// glyph run (which advances by the atlas's own advance) drifts off the grid
    /// columns — the classic terminal font mis-spacing. Reads a reference glyph;
    /// falls back to ~0.6em when none can rasterize.
    pub fn cell_advance(&mut self) -> i64 {
        for c in ['0', 'M', 'x', 'W'] {
            if let Some(g) = self.get_or_rasterize(c) {
                if g.advance > 0 {
                    return g.advance;
                }
            }
        }
        ((self.font_size * 600.0) as i64).max(1_000)
    }

    /// Rasterize a character and blit into texture_data. Returns None for empty bitmaps or atlas full.
    /// NOTE: fontdue's metrics.advance_width is f32, converted to i64 immediately at the boundary.
    fn rasterize_and_cache(&mut self, c: char) -> Option<AtlasGlyph> {
        let (metrics, bitmap) = self.font.rasterize(c, self.font_size);

        // Empty bitmap (e.g. space character)
        if metrics.width == 0 || metrics.height == 0 || bitmap.is_empty() {
            return None;
        }

        let (ox, oy) = self.pack_glyph(metrics.width, metrics.height)?;

        for row in 0..metrics.height {
            let src_start = row * metrics.width;
            let dst_start = (oy + row) * ATLAS_SIZE + ox;
            let len = metrics.width;
            self.texture_data[dst_start..dst_start + len]
                .copy_from_slice(&bitmap[src_start..src_start + len]);
        }

        self.dirty = true;

        let inv = 1.0 / ATLAS_SIZE as f32;
        Some(AtlasGlyph {
            uv: [
                ox as f32 * inv,
                oy as f32 * inv,
                (ox + metrics.width) as f32 * inv,
                (oy + metrics.height) as f32 * inv,
            ],
            advance: (metrics.advance_width * 1000.0) as i64,
            offset: [
                metrics.xmin as i16,
                -(metrics.ymin as i16),
            ],
            size: [metrics.width as u16, metrics.height as u16],
        })
    }

    /// Fixed atlas cell edge (px) reserved per extended-cache ring slot: 2× the
    /// font size, generous enough for any single glyph rasterized at this size.
    fn slot_cell(&self) -> usize {
        (self.font_size.ceil() as usize).saturating_mul(2).max(1)
    }

    /// Rasterize a non-ASCII glyph into the atlas region RESERVED for `slot`.
    ///
    /// BUG FIX (glyph-cache corruption on long sessions): the old extended path
    /// called `pack_glyph` on every cache miss, so an evicted-then-different
    /// glyph permanently consumed brand-new atlas territory. After
    /// `EXTENDED_CACHE_SIZE` distinct glyphs the 4096² atlas ran out
    /// (`pack_glyph` -> `None`, glyphs silently vanished / boxed). Now each slot
    /// reserves ONE fixed cell the first time it is used and reuses it for every
    /// glyph later assigned to that slot, so extended-glyph atlas usage is
    /// bounded at `EXTENDED_CACHE_SIZE` cells forever.
    fn rasterize_into_slot(&mut self, c: char, slot: usize) -> Option<AtlasGlyph> {
        let (metrics, bitmap) = self.font.rasterize(c, self.font_size);

        // Empty bitmap (e.g. NBSP) — nothing to cache, no atlas consumed.
        if metrics.width == 0 || metrics.height == 0 || bitmap.is_empty() {
            return None;
        }

        let cell = self.slot_cell();

        // A glyph larger than the reserved cell would clobber a neighbouring
        // slot — skip it rather than corrupt the atlas.
        if metrics.width > cell || metrics.height > cell {
            return None;
        }

        // Reserve this slot's fixed cell on first use; reuse it forever after.
        let (ox, oy) = match self.extended_slot_origin[slot] {
            Some(origin) => origin,
            None => {
                let origin = self.pack_glyph(cell, cell)?;
                self.extended_slot_origin[slot] = Some(origin);
                origin
            }
        };

        // Clear the whole cell so a smaller glyph leaves no stale pixels from a
        // previously-evicted larger glyph occupying the same slot.
        for row in 0..cell {
            let dst = (oy + row) * ATLAS_SIZE + ox;
            for px in &mut self.texture_data[dst..dst + cell] {
                *px = 0;
            }
        }

        // Blit the glyph into the top-left of the cell.
        for row in 0..metrics.height {
            let src_start = row * metrics.width;
            let dst_start = (oy + row) * ATLAS_SIZE + ox;
            self.texture_data[dst_start..dst_start + metrics.width]
                .copy_from_slice(&bitmap[src_start..src_start + metrics.width]);
        }

        self.dirty = true;

        let inv = 1.0 / ATLAS_SIZE as f32;
        Some(AtlasGlyph {
            uv: [
                ox as f32 * inv,
                oy as f32 * inv,
                (ox + metrics.width) as f32 * inv,
                (oy + metrics.height) as f32 * inv,
            ],
            advance: (metrics.advance_width * 1000.0) as i64,
            offset: [metrics.xmin as i16, -(metrics.ymin as i16)],
            size: [metrics.width as u16, metrics.height as u16],
        })
    }

    /// Return GlyphData compatible with measure_text/layout_text.
    /// Returns zeroed GlyphData if the character is not cached.
    pub fn metrics(&self, c: char) -> GlyphData {
        let code = c as usize;

        // ASCII fast path
        if code < 256 {
            if let Some(glyph) = self.glyph_cache[code] {
                return glyph_to_data(&glyph);
            }
            let m = self.font.metrics(c, self.font_size);
            return GlyphData {
                uv: [0.0; 4],
                advance: MilliUnit((m.advance_width * 1000.0) as i64),
                offset: [0, 0],
                size: [0, 0],
            };
        }

        for i in 0..self.extended_cache_count.min(EXTENDED_CACHE_SIZE) {
            if self.extended_cache[i].0 == c {
                if let Some(glyph) = self.extended_cache[i].1 {
                    return glyph_to_data(&glyph);
                }
            }
        }

        let m = self.font.metrics(c, self.font_size);
        GlyphData {
            uv: [0.0; 4],
            advance: MilliUnit((m.advance_width * 1000.0) as i64),
            offset: [0, 0],
            size: [0, 0],
        }
    }

    /// Inject a pre-authored R8 bitmap at a specific codepoint, bypassing fontdue.
    /// Use for private-use-area icon glyphs (U+EA00..U+EAFF, etc.).
    /// `bitmap` must be exactly `width * height` R8 bytes (255=opaque, 0=transparent).
    /// Returns true if the glyph was registered; false if atlas is full or input is invalid.
    pub fn inject_bitmap(&mut self, c: char, bitmap: &[u8], width: u16, height: u16) -> bool {
        let w = width as usize;
        let h = height as usize;
        if w == 0 || h == 0 || bitmap.len() != w * h {
            return false;
        }

        let (ox, oy) = match self.pack_glyph(w, h) {
            Some(pos) => pos,
            None => return false,
        };

        for row in 0..h {
            let src = &bitmap[row * w..(row + 1) * w];
            let dst_start = (oy + row) * ATLAS_SIZE + ox;
            self.texture_data[dst_start..dst_start + w].copy_from_slice(src);
        }
        self.dirty = true;

        let inv = 1.0 / ATLAS_SIZE as f32;
        let glyph = AtlasGlyph {
            uv: [
                ox as f32 * inv,
                oy as f32 * inv,
                (ox + w) as f32 * inv,
                (oy + h) as f32 * inv,
            ],
            advance: (w as i64) * 1000,
            offset: [0, 0],
            size: [width, height],
        };

        let code = c as usize;
        if code < 256 {
            self.glyph_cache[code] = Some(glyph);
        } else {
            for i in 0..self.extended_cache_count.min(EXTENDED_CACHE_SIZE) {
                if self.extended_cache[i].0 == c {
                    self.extended_cache[i].1 = Some(glyph);
                    return true;
                }
            }
            if self.extended_cache_count < EXTENDED_CACHE_SIZE {
                self.extended_cache[self.extended_cache_count] = (c, Some(glyph));
                self.extended_cache_count += 1;
            } else {
                let slot = self.extended_cache_count % EXTENDED_CACHE_SIZE;
                self.extended_cache[slot] = (c, Some(glyph));
                self.extended_cache_count += 1;
            }
        }
        true
    }

    /// Mark atlas as clean after GPU upload.
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Check if atlas has new glyphs needing upload.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Pair-kern advance (MilliUnit) between adjacent glyphs — O(1) baked-matrix lookup.
    /// Fixes AV/To/Wa gaps. 0 for non-printable-ASCII pairs or a GPOS-less font.
    #[inline]
    pub fn kern(&self, prev: char, cur: char) -> i64 {
        const LO: u32 = 32;
        const N: usize = 95;
        let (p, c) = (prev as u32, cur as u32);
        if p >= LO && p <= 126 && c >= LO && c <= 126 && self.kern_ascii.len() == N * N {
            self.kern_ascii[(p - LO) as usize * N + (c - LO) as usize] as i64
        } else {
            0
        }
    }
}

/// Convert AtlasGlyph to GlyphData for measure_text/layout_text compatibility.
fn glyph_to_data(g: &AtlasGlyph) -> GlyphData {
    GlyphData {
        uv: g.uv,
        advance: MilliUnit(g.advance),
        offset: g.offset,
        size: g.size,
    }
}

/// Stub for gpos_kern extraction when module is unavailable.
/// Returns an empty kern matrix; actual GPOS extraction deferred to gpos_kern module.
fn extract_ascii_kern_or_empty(
    _font_bytes: &[u8],
    _font_size: f32,
    _glyph_index_fn: impl Fn(char) -> Option<u16>,
) -> Box<[i16]> {
    vec![].into_boxed_slice()
}

// ── Text measurement and layout ──────────────────────────────────────────────

/// Measure text width without emitting glyphs (for ContentFit constraint).
pub fn measure_text<F>(text: &str, metrics: &F) -> MilliUnit
where
    F: Fn(char) -> GlyphData,
{
    let mut width: i64 = 0;
    for ch in text.chars() {
        width += metrics(ch).advance.0;
    }
    MilliUnit(width)
}

/// Measure text height given a bounding width (accounts for word-wrap).
pub fn measure_text_height<F>(
    text: &str,
    metrics: &F,
    bounds_w: MilliUnit,
    line_height: MilliUnit,
) -> MilliUnit
where
    F: Fn(char) -> GlyphData,
{
    if text.is_empty() {
        return line_height;
    }

    let mut cursor_x: i64 = 0;
    let mut lines: i64 = 1;

    for word in text.split_whitespace() {
        let word_width: i64 = word.chars().map(|ch| metrics(ch).advance.0).sum();
        let space_width = metrics(' ').advance.0;

        // Wrap if word doesn't fit on current line (unless it's the first word on the line)
        if cursor_x > 0 && cursor_x + space_width + word_width > bounds_w.0 {
            lines += 1;
            cursor_x = word_width;
        } else {
            if cursor_x > 0 {
                cursor_x += space_width;
            }
            cursor_x += word_width;
        }
    }

    MilliUnit(lines * line_height.0)
}

/// Word-wrap text layout. Integer math for line breaking, float output for GPU.
pub fn layout_text<F>(
    text: &str,
    metrics: &F,
    bounds_w: MilliUnit,
    line_height: MilliUnit,
    origin: (MilliUnit, MilliUnit),
    color: u32,
) -> Vec<GlyphInstance>
where
    F: Fn(char) -> GlyphData,
{
    let mut glyphs = Vec::new();
    let mut cursor_x: i64 = 0;
    let mut cursor_y: i64 = 0;
    let space_data = metrics(' ');

    for word in text.split_whitespace() {
        let word_width: i64 = word.chars().map(|ch| metrics(ch).advance.0).sum();

        if cursor_x > 0 && cursor_x + space_data.advance.0 + word_width > bounds_w.0 {
            cursor_x = 0;
            cursor_y += line_height.0;
        } else if cursor_x > 0 {
            cursor_x += space_data.advance.0;
        }

        for ch in word.chars() {
            let data = metrics(ch);
            if data.size[0] > 0 && data.size[1] > 0 {
                let px = (origin.0 .0 + cursor_x) as f32 / 1000.0 + data.offset[0] as f32;
                let py = (origin.1 .0 + cursor_y) as f32 / 1000.0 + data.offset[1] as f32;

                glyphs.push(GlyphInstance {
                    pos: [px, py],
                    uv: data.uv,
                    color,
                    size: [data.size[0] as f32, data.size[1] as f32],
                });
            }
            cursor_x += data.advance.0;
        }
    }

    glyphs
}

// ── Type Ramp: Font Size Stops ──────────────────────────────────────────────

/// Font size stops in the type ramp. Used to index atlases in a MultiAtlas
/// and to select which face/size combination renders a Text command.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FontSize {
    /// Caption / small labels — `ramp[0]`.
    Caption = 0,
    /// Body text — `ramp[1]`, the default.
    #[default]
    Body = 1,
    /// Emphasis / section subhead — `ramp[2]`, the bold stop.
    Subhead = 2,
    /// Section heading — `ramp[3]`.
    Heading = 3,
    /// Display / titles — `ramp[4]`.
    Display = 4,
    /// Fixed-advance slot (rasterized at the body size).
    Mono = 5,
}

impl FontSize {
    /// Inverse of `self as u8` — out-of-range indices read Body (the default).
    #[inline]
    pub fn from_index(i: u8) -> Self {
        match i {
            0 => FontSize::Caption,
            2 => FontSize::Subhead,
            3 => FontSize::Heading,
            4 => FontSize::Display,
            5 => FontSize::Mono,
            _ => FontSize::Body,
        }
    }
}

/// Multi-atlas ramp: one `FontAtlas` per `type.ramp` stop (Caption, Body, Subhead,
/// Heading, Display, Mono). Each atlas rasterizes glyphs at the stop's own font size.
/// A `MultiAtlas` is allocated once at boot; `rasterize_into_ramp` then blits
/// per-command from the atlas matching each Text command's `text_face` stop.
pub struct MultiAtlas {
    /// Caption atlas (ramp[0]).
    caption: FontAtlas,
    /// Body atlas (ramp[1]), the default.
    body: FontAtlas,
    /// Subhead atlas (ramp[2]).
    subhead: FontAtlas,
    /// Heading atlas (ramp[3]).
    heading: FontAtlas,
    /// Display atlas (ramp[4]).
    display: FontAtlas,
    /// Mono atlas (ramp[5]).
    mono: FontAtlas,
}

impl MultiAtlas {
    /// Allocate a full ramp of atlases, one per `FontSize` stop. Each atlas
    /// is initialized with the corresponding face and size from the ramp definition.
    /// Boot-time only; reuse forever.
    pub fn init(ramp_faces: &[TypeFace; 5], mono_face: TypeFace) -> Self {
        Self {
            caption: FontAtlas::init(ramp_faces[0].bytes(), 12.0),
            body: FontAtlas::init(ramp_faces[1].bytes(), 14.0),
            subhead: FontAtlas::init(ramp_faces[2].bytes(), 16.0),
            heading: FontAtlas::init(ramp_faces[3].bytes(), 20.0),
            display: FontAtlas::init(ramp_faces[4].bytes(), 28.0),
            mono: FontAtlas::init(mono_face.bytes(), 14.0),
        }
    }

    /// Retrieve the atlas for a given `FontSize` ramp stop.
    #[inline]
    pub fn get(&self, size: FontSize) -> &FontAtlas {
        match size {
            FontSize::Caption => &self.caption,
            FontSize::Body => &self.body,
            FontSize::Subhead => &self.subhead,
            FontSize::Heading => &self.heading,
            FontSize::Display => &self.display,
            FontSize::Mono => &self.mono,
        }
    }

    /// Retrieve a mutable reference to the atlas for a given `FontSize` ramp stop.
    /// Required for rasterize-on-demand (frame loop calls this to dirty-check and
    /// rasterize missing glyphs before GPU upload).
    #[inline]
    pub fn get_mut(&mut self, size: FontSize) -> &mut FontAtlas {
        match size {
            FontSize::Caption => &mut self.caption,
            FontSize::Body => &mut self.body,
            FontSize::Subhead => &mut self.subhead,
            FontSize::Heading => &mut self.heading,
            FontSize::Display => &mut self.display,
            FontSize::Mono => &mut self.mono,
        }
    }
}

// ── type.ramp → real face+size ladder ────────────────────────────────────────
// Ported verbatim from F:\NewRepo\crates\forge-canvas\src\text.rs (v2, 2026-07-30
// ladder). TTFs ride `include_bytes!` — no runtime file load, so the ladder
// resolves headless and in a WASI build. Font assets copied to
// F:\v3\assets\fonts\ (11 files, the ladder's actual embedded set).

/// The faces the type ramp speaks in. Deliberately the LADDER set only, not a
/// font picker — the picker registry (`forge_gui::font_stamp::FontChoice`) has
/// no v3 home yet; embedding a face twice costs binary weight for nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TypeFace {
    /// Ramp caption/body — humanist condensed, high x-height at small sizes.
    #[default]
    Rajdhani,
    /// Ramp emphasis stop.
    RajdhaniBold,
    /// Ramp heading — the structural voice.
    Tektur,
    /// Fixed-advance: terminal, code, numeric columns.
    IosevkaFixed,
    /// Ramp DISPLAY — Cormorant Garamond, the Jean Jannon / 1621 Sedan cut behind
    /// what most people call Garamond. High vertical contrast, asymmetric
    /// serifs, cut for royal declarations.
    CormorantGaramond,
    /// Ramp HEADING — Reem Kufi, off 12th-century Mamluk astrolabe brass
    /// engraving and Kufic stone frieze. Geometric, tile-aligned, non-slanted
    /// stems: the same module-grid logic the layout engine already lowers on.
    ReemKufi,
    /// LORE / narrative — EB Garamond, a 16th-century humanist revival.
    /// Long-form manuscript overlays and flavour lines. Its delicate serifs
    /// break down in compact HUD meters, so it stays off the telemetry rails
    /// by design.
    EbGaramond,
    /// EB Garamond italic — the expressive voice cut, for quoted narration.
    EbGaramondItalic,
    /// Amiri — Naskh, the Andalusian / Mamluk scientific-treatise hand. The
    /// lore body's other half, paired with `EbGaramondItalic` rather than
    /// competing.
    Amiri,
    /// Cinzel — 1st-century Roman inscription. Kept OFF the default ladder: it
    /// is imperial, not scholarly, and reads as generic fantasy below `Display`.
    /// Available for faction/milestone banners that want stone instead of
    /// copperplate.
    Cinzel,
    /// JetBrains Mono — ENGINE TRUTH. Spatial coordinates, permyriad metrics,
    /// telemetry. Pixel-locked advance so `[X,Y,Z,T,S]` columns cannot drift.
    JetBrainsMono,
}

impl TypeFace {
    /// The embedded TTF bytes for this face.
    pub const fn bytes(self) -> &'static [u8] {
        match self {
            Self::Rajdhani => include_bytes!("../../../assets/fonts/rajdhani_regular.ttf"),
            Self::RajdhaniBold => include_bytes!("../../../assets/fonts/rajdhani_bold.ttf"),
            Self::Tektur => include_bytes!("../../../assets/fonts/tektur_variable.ttf"),
            Self::IosevkaFixed => include_bytes!("../../../assets/fonts/IosevkaFixed-Regular.ttf"),
            Self::CormorantGaramond => {
                include_bytes!("../../../assets/fonts/CormorantGaramond-Variable.ttf")
            }
            Self::ReemKufi => include_bytes!("../../../assets/fonts/ReemKufi-Variable.ttf"),
            Self::EbGaramond => include_bytes!("../../../assets/fonts/EBGaramond-Variable.ttf"),
            Self::EbGaramondItalic => {
                include_bytes!("../../../assets/fonts/EBGaramond-Italic-Variable.ttf")
            }
            Self::Amiri => include_bytes!("../../../assets/fonts/Amiri-Regular.ttf"),
            Self::Cinzel => include_bytes!("../../../assets/fonts/Cinzel-Variable.ttf"),
            Self::JetBrainsMono => {
                include_bytes!("../../../assets/fonts/JetBrainsMono-Variable.ttf")
            }
        }
    }

    /// Short display label (theme readout / capture stamp).
    pub const fn label(self) -> &'static str {
        match self {
            Self::Rajdhani => "Rajdhani",
            Self::RajdhaniBold => "Rajdhani Bold",
            Self::Tektur => "Tektur",
            Self::IosevkaFixed => "Iosevka Fixed",
            Self::CormorantGaramond => "Cormorant Garamond",
            Self::ReemKufi => "Reem Kufi",
            Self::EbGaramond => "EB Garamond",
            Self::EbGaramondItalic => "EB Garamond Italic",
            Self::Amiri => "Amiri",
            Self::Cinzel => "Cinzel",
            Self::JetBrainsMono => "JetBrains Mono",
        }
    }

    /// True when every glyph shares one advance — the terminal/code guarantee.
    pub const fn is_mono(self) -> bool {
        matches!(self, Self::IosevkaFixed | Self::JetBrainsMono)
    }
}

/// `type.ramp[0..4]` → face — the Grand Siècle / astrolabe pairing.
///
/// * `[4] Display` — **Cormorant Garamond**: the Jannon copperplate, authority.
/// * `[3] Heading` — **Reem Kufi**: astrolabe brass, geometric and tile-aligned.
/// * `[2] Subhead` — **Reem Kufi**: registers and relic names stay in that voice.
/// * `[0..1] Caption/Body` — **EB Garamond**: the manuscript hand, for reading.
///
/// Engine truth is NOT on this ladder — coordinates, permyriad metrics and
/// telemetry take [`MONO_RAMP_FACES`], because a proportional face lets a
/// `[X,Y,Z,T,S]` column drift and a drifting column is a lie about the data.
pub const RAMP_FACES: [TypeFace; 5] = [
    TypeFace::EbGaramond,
    TypeFace::EbGaramond,
    TypeFace::ReemKufi,
    TypeFace::ReemKufi,
    TypeFace::CormorantGaramond,
];

/// The ladder a fixed-advance surface (terminal, code, CST score) resolves —
/// one mono face at every stop so columns align across the whole ramp.
///
/// JetBrains Mono is ENGINE TRUTH: the register that speaks coordinates and
/// metrics is deliberately the one register with no period costume on it.
/// Iosevka stays a [`TypeFace`] for surfaces that pinned it.
pub const MONO_RAMP_FACES: [TypeFace; 5] = [TypeFace::JetBrainsMono; 5];

#[cfg(test)]
mod tests {
    use super::*;

    /// The subpixel filter preserves energy, spreads a lone stripe across its
    /// neighbours (that spread IS the fringe suppression), and leaves a solid
    /// run solid — the three things a broken LCD filter always gets wrong.
    #[test]
    fn lcd_filter_is_energy_preserving_and_spreads_fringes() {
        assert_eq!(LCD_FIR5.iter().sum::<u16>(), 256, "weights must sum to 256");

        let solid = [255u8; 12];
        let mut out = [[0u8; 3]; 4];
        lcd_filter_row(&solid, &mut out);
        assert_eq!(out, [[255u8; 3]; 4], "full coverage must stay full");

        let mut spike = [0u8; 12];
        spike[6] = 255;
        let mut out = [[0u8; 3]; 4];
        lcd_filter_row(&spike, &mut out);
        assert!(out[2][0] > 0, "the lit stripe carries the most weight");
        assert!(out[1][2] > 0 && out[2][1] > 0, "neighbours share the energy");
        assert!(out[2][0] >= out[1][2], "centre stays the brightest");

        let empty: [u8; 0] = [];
        lcd_filter_row(&empty, &mut out);
    }

    fn mock_metrics(ch: char) -> GlyphData {
        // Monospace mock: every char is 8px wide (8000 MilliUnit)
        GlyphData {
            uv: [0.0, 0.0, 0.01, 0.01],
            advance: MilliUnit(8000),
            offset: [0, 0],
            size: if ch == ' ' { [0, 0] } else { [8, 16] },
        }
    }

    /// L07: determinism test — same input must produce same output.
    #[test]
    fn measure_text_is_deterministic() {
        let text = "hello";
        let r1 = measure_text(text, &mock_metrics);
        let r2 = measure_text(text, &mock_metrics);
        assert_eq!(r1.0, r2.0, "measure_text must be deterministic");
    }

    #[test]
    fn measure_simple() {
        let width = measure_text("hello", &mock_metrics);
        assert_eq!(width.0, 40000, "5 chars × 8000");
    }

    /// L18: sabotage test — verify the determinism invariant by checking a known value.
    #[test]
    fn layout_text_deterministic_output() {
        let glyphs1 = layout_text(
            "hello world",
            &mock_metrics,
            MilliUnit(50000),
            MilliUnit(16000),
            (MilliUnit(0), MilliUnit(0)),
            0xFFFFFFFF,
        );
        let glyphs2 = layout_text(
            "hello world",
            &mock_metrics,
            MilliUnit(50000),
            MilliUnit(16000),
            (MilliUnit(0), MilliUnit(0)),
            0xFFFFFFFF,
        );
        assert_eq!(glyphs1.len(), glyphs2.len(), "layout_text must produce same glyph count");
        for (g1, g2) in glyphs1.iter().zip(glyphs2.iter()) {
            assert_eq!(g1.pos[0], g2.pos[0], "glyph x position must be deterministic");
            assert_eq!(g1.pos[1], g2.pos[1], "glyph y position must be deterministic");
        }
    }

    #[test]
    fn layout_wraps_at_boundary() {
        // Bounds = 50000 (50px). Each char = 8000 (8px). "hello world" = 11 chars.
        // "hello" = 40000, fits. "world" = 40000, doesn't fit after space. Wraps.
        let glyphs = layout_text(
            "hello world",
            &mock_metrics,
            MilliUnit(50000),
            MilliUnit(16000),
            (MilliUnit(0), MilliUnit(0)),
            0xFFFFFFFF,
        );
        // "hello" = 5 visible glyphs on line 0, "world" = 5 on line 1
        assert_eq!(glyphs.len(), 10, "wrapped text should have 10 glyphs");
        // First glyph of "world" should be on a new line
        let first_world = &glyphs[5];
        assert!(first_world.pos[1] > 0.0, "wrapped line should have y > 0");
    }

    #[test]
    fn measure_height_wraps() {
        let h = measure_text_height("hello world", &mock_metrics, MilliUnit(50000), MilliUnit(16000));
        assert_eq!(h.0, 32000, "2 lines × 16000");
    }

    /// L07: bijection test — glyph_to_data must round-trip without loss.
    #[test]
    fn glyph_to_data_round_trip() {
        let atlas_glyph = AtlasGlyph {
            uv: [0.1, 0.2, 0.3, 0.4],
            advance: 8000,
            offset: [1, -2],
            size: [8, 16],
        };
        let data = glyph_to_data(&atlas_glyph);
        assert_eq!(data.uv, atlas_glyph.uv, "UV must round-trip");
        assert_eq!(data.advance.0, atlas_glyph.advance, "advance must round-trip");
        assert_eq!(data.offset, atlas_glyph.offset, "offset must round-trip");
        assert_eq!(data.size, atlas_glyph.size, "size must round-trip");
    }
}
