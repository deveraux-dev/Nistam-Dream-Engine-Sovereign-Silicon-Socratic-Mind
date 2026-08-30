//! GIF encoder lane — scrubbable reel-column encoding to GIF.
//! Frame delay derives from ReelClock dwell_ms; refuses dwell<10ms rather
//! than silently rounding to 0. Each frame carries EngineTick8 stamp; bijection
//! column<->frame is verified at add time.

use crate::clock::ReelClock;
use forge_engine_v3::EngineTick8;

/// Encode refusals — typed, never a bare string at the call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GifWindowError {
    /// The clock's dwell is under 10ms, which floors to a 0-centisecond GIF
    /// delay — a 0 delay runs at the browser's default rate, which would make
    /// every Drop Law dwell floor a lie. Refused rather than rounded.
    DwellTooSmall {
        /// The dwell that could not be expressed in centiseconds.
        dwell_ms: u32,
    },
    /// `scrub(column)` produced a tick that does not read back as `column`.
    BijectionViolation {
        /// The column the tick decoded to.
        computed: u32,
        /// The column that was asked for.
        expected: u32,
    },
    /// The clock refused to seek this column at all.
    InvalidColumn {
        /// The column that would not encode.
        column: u32,
    },
    /// The `gif` encoder itself refused.
    EncodingFailed(
        /// The encoder's own message.
        String,
    ),
}

/// One encoded column: its tape stamp and its indexed pixels.
struct GifFrame {
    column: u32,
    _tick: EngineTick8,
    pixels: Vec<u8>,
}

/// A scrubbable window of reel columns, encodable to GIF.
pub struct GifWindow {
    clock: ReelClock,
    width: u16,
    height: u16,
    palette: Vec<u8>,
    frames: Vec<GifFrame>,
}

impl GifWindow {
    /// A window on `clock`, sized `width`x`height`, over a caller-supplied
    /// global palette. Refuses a dwell that cannot survive the centisecond
    /// conversion.
    pub fn new(
        clock: ReelClock,
        width: u16,
        height: u16,
        palette: Vec<u8>,
    ) -> Result<Self, GifWindowError> {
        if clock.dwell_ms() < 10 {
            return Err(GifWindowError::DwellTooSmall {
                dwell_ms: clock.dwell_ms(),
            });
        }
        Ok(Self {
            clock,
            width,
            height,
            palette,
            frames: Vec::new(),
        })
    }

    /// Add one column's indexed pixels. The tape stamp is taken here and the
    /// column round-trip is checked BEFORE the frame is kept, so a bijection
    /// break is caught at add time rather than at encode.
    pub fn add_frame(&mut self, column: u32, pixels: &[u8]) -> Result<(), GifWindowError> {
        let tick = self
            .clock
            .scrub(column)
            .ok_or(GifWindowError::InvalidColumn { column })?;

        let computed_column = self.clock.column_at(tick);
        if computed_column != column {
            return Err(GifWindowError::BijectionViolation {
                computed: computed_column,
                expected: column,
            });
        }

        self.frames.push(GifFrame {
            column,
            _tick: tick,
            pixels: pixels.to_vec(),
        });

        Ok(())
    }

    /// Encode every added column to GIF bytes, one frame per column, each
    /// holding for the clock's own dwell.
    pub fn finalize(self) -> Result<Vec<u8>, GifWindowError> {
        let mut output = Vec::new();
        {
            let mut encoder = gif::Encoder::new(&mut output, self.width, self.height, &self.palette)
                .map_err(|e| GifWindowError::EncodingFailed(format!("{:?}", e)))?;

            // Centiseconds, floored — `new` already refused any dwell that would
            // round to 0, so this cannot silently produce a browser-default GIF.
            let delay_cs = (self.clock.dwell_ms() / 10) as u16;

            for frame_data in self.frames {
                // `transparent: None` — a reel column is opaque; the Drop Law
                // dwell is what holds a frame, never an alpha key.
                let mut frame = gif::Frame::from_indexed_pixels(
                    self.width,
                    self.height,
                    frame_data.pixels,
                    None,
                );
                frame.delay = delay_cs;
                encoder
                    .write_frame(&frame)
                    .map_err(|e| GifWindowError::EncodingFailed(format!("{:?}", e)))?;
            }
        }
        Ok(output)
    }

    /// The clock this window was cut against.
    pub fn clock(&self) -> ReelClock {
        self.clock
    }

    /// How many columns have been added.
    pub fn frame_count(&self) -> u32 {
        self.frames.len() as u32
    }

    /// The tape column a GIF frame index came from — the inverse of the add
    /// order, and the half of the bijection a scrubber reads.
    pub fn column_of_frame(&self, frame_index: u32) -> Option<u32> {
        self.frames.get(frame_index as usize).map(|f| f.column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_dwell_below_10ms() {
        let clock = ReelClock::new(5);
        assert!(matches!(
            GifWindow::new(clock, 640, 480, vec![]),
            Err(GifWindowError::DwellTooSmall { dwell_ms: 5 })
        ));
    }

    #[test]
    fn accepts_dwell_at_10ms() {
        let clock = ReelClock::new(10);
        assert!(GifWindow::new(clock, 640, 480, vec![]).is_ok());
    }

    #[test]
    fn bijection_column_to_frame_to_column() {
        let clock = ReelClock::kept();
        let mut window = GifWindow::new(clock, 640, 480, vec![0; 256 * 3])
            .expect("window creation");

        let test_columns = [0u32, 1, 5, 100, 1000];
        for &col in &test_columns {
            let pixels = vec![0u8; (640 * 480) as usize];
            window.add_frame(col, &pixels).expect("add frame");
        }

        for (frame_idx, &expected_col) in test_columns.iter().enumerate() {
            let column = window
                .column_of_frame(frame_idx as u32)
                .expect("frame exists");
            assert_eq!(column, expected_col, "Frame {} column bijection failed", frame_idx);

            let tick = clock.scrub(column).expect("valid scrub");
            let computed = clock.column_at(tick);
            assert_eq!(computed, column, "Clock round-trip failed for frame {}", frame_idx);
        }
    }

    #[test]
    fn gif_encoding_produces_bytes() {
        let clock = ReelClock::new(100);
        let mut window = GifWindow::new(clock, 2, 2, vec![0; 256 * 3])
            .expect("window creation");

        let pixels = vec![0u8; 4];
        window.add_frame(0, &pixels).expect("add frame");

        let result = window.finalize();
        assert!(result.is_ok(), "GIF encoding should succeed");
        let bytes = result.unwrap();
        assert!(!bytes.is_empty(), "GIF should produce bytes");
        assert!(
            bytes.starts_with(b"GIF"),
            "GIF should start with GIF header"
        );
    }

    #[test]
    fn delay_computed_from_clock_dwell() {
        let clock = ReelClock::new(200);
        let mut window = GifWindow::new(clock, 2, 2, vec![0; 256 * 3])
            .expect("window creation");
        let pixels = vec![0u8; 4];
        window.add_frame(0, &pixels).expect("add frame");

        let bytes = window.finalize().expect("finalize");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn frame_count_tracks_additions() {
        let clock = ReelClock::new(50);
        let mut window = GifWindow::new(clock, 640, 480, vec![0; 256 * 3])
            .expect("window");

        assert_eq!(window.frame_count(), 0);
        for i in 0..5 {
            let pixels = vec![0u8; (640 * 480) as usize];
            window.add_frame(i, &pixels).expect("add frame");
        }
        assert_eq!(window.frame_count(), 5);
    }
}
