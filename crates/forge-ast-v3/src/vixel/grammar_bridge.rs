//! # grammar_bridge.rs — Hand-Rolled Recursive Descent Parser for VixiScript
//!
//! Parses `.vixel` source files into a typed `VixelAst`. No external parser
//! dependencies (no nom, no pest, no tree-sitter at this stage).
//!
//! **Grammar constructs:**
//! - `material "name" { ... }` blocks
//! - `spawn_*("name", ...)` calls
//! - `rule "name" { when: ..., then: ..., tick_delay: N }` blocks
//! - `set_*("name", ...)` / `set_*([ ... ], ...)` calls
//!
//! **Constraints:**
//! - `number` rule: integers only (`[0-9]+` or `0x[0-9a-fA-F]+`). Floats rejected.
//! - Parse errors include file path and line number.
//! - Never panics on malformed input — returns `ParseError`.

use super::{
    AcrylicDef, AtomDef, AutomataDef, AutomataType, BrushDef, EnvironmentDef, EnvironmentType,
    LayersDef, MaterialDef, ParseError, PressureDef, SpatialDef, UiDef, ViewportDef, VixelAst,
    // Token system types
    ThemeDef, ThemeLayer, TokenDef, ColorValue, RepeatDir, SpringDef, ParticleDef,
    FillDef, FillDir, EmitterType, ConditionalRule, CmpOp, BindTarget,
};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a single VixiScript source string into a `VixelAst`.
///
/// `file_name` is used for error reporting only.
pub fn parse_vixel_source(source: &str, file_name: &str) -> Result<VixelAst, ParseError> {
    let mut parser = Parser::new(source, file_name);
    parser.parse_source()
}

// ---------------------------------------------------------------------------
// Tokenizer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Token {
    /// Keyword or identifier: material, rule, spawn_grid, etc.
    Ident(String),
    /// String literal (without quotes).
    StringLit(String),
    /// Integer literal.
    IntLit(u64),
    /// Permyriad literal (integer with `p` suffix, e.g. `5000p` = 50%).
    PermyriadLit(i32),
    /// Hex literal (0x...).
    HexLit(u64),
    /// Punctuation / operators.
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Colon,
    Comma,
    Semicolon,
    Dot,
    Gt,
    Lt,
    Ampersand2, // &&
    Pipe2,      // ||
    Bang,       // !
    Eq,         // =  (unused in grammar but tokenized for safety)
    Eq2,        // ==
    Minus,      // -
    Plus,       // +
    /// Token reference: $name
    TokenRef(String),
    /// End of file.
    Eof,
}

#[derive(Debug, Clone)]
struct Located {
    token: Token,
    line: usize,
}

struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
    line: usize,
    file: &'a str,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str, file: &'a str) -> Self {
        Self {
            src: src.as_bytes(),
            pos: 0,
            line: 1,
            file,
        }
    }

    fn peek_byte(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let b = self.src.get(self.pos).copied()?;
        self.pos += 1;
        if b == b'\n' {
            self.line += 1;
        }
        Some(b)
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            while let Some(b) = self.peek_byte() {
                if b == b' ' || b == b'\t' || b == b'\r' || b == b'\n' {
                    self.advance();
                } else {
                    break;
                }
            }
            if self.pos + 1 < self.src.len()
                && self.src[self.pos] == b'/'
                && self.src[self.pos + 1] == b'/'
            {
                while let Some(b) = self.advance() {
                    if b == b'\n' {
                        break;
                    }
                }
                continue;
            }
            break;
        }
    }

    fn err(&self, msg: impl Into<String>) -> ParseError {
        ParseError {
            file: self.file.to_string(),
            line: self.line,
            message: msg.into(),
        }
    }

    fn next_token(&mut self) -> Result<Located, ParseError> {
        self.skip_whitespace_and_comments();

        let line = self.line;

        let Some(b) = self.peek_byte() else {
            return Ok(Located {
                token: Token::Eof,
                line,
            });
        };

        if b == b'"' {
            return self.lex_string(line);
        }
        if b.is_ascii_digit() {
            return self.lex_number(line);
        }
        if b.is_ascii_alphabetic() || b == b'_' {
            return self.lex_ident(line);
        }
        self.advance();
        let tok = match b {
            b'{' => Token::LBrace,
            b'}' => Token::RBrace,
            b'(' => Token::LParen,
            b')' => Token::RParen,
            b'[' => Token::LBracket,
            b']' => Token::RBracket,
            b':' => Token::Colon,
            b',' => Token::Comma,
            b';' => Token::Semicolon,
            b'.' => Token::Dot,
            b'>' => Token::Gt,
            b'<' => Token::Lt,
            b'!' => Token::Bang,
            b'-' => Token::Minus,
            b'+' => Token::Plus,
            b'$' => {
                let start = self.pos;
                while let Some(c) = self.peek_byte() {
                    if c.is_ascii_alphanumeric() || c == b'_' {
                        self.advance();
                    } else {
                        break;
                    }
                }
                if self.pos == start {
                    return Err(self.err("expected identifier after '$'"));
                }
                let name = std::str::from_utf8(&self.src[start..self.pos])
                    .map_err(|_| self.err("invalid UTF-8 in token reference"))?;
                Token::TokenRef(name.to_string())
            }
            b'&' => {
                if self.peek_byte() == Some(b'&') {
                    self.advance();
                    Token::Ampersand2
                } else {
                    return Err(self.err("unexpected '&' — did you mean '&&'?"));
                }
            }
            b'|' => {
                if self.peek_byte() == Some(b'|') {
                    self.advance();
                    Token::Pipe2
                } else {
                    return Err(self.err("unexpected '|' — did you mean '||'?"));
                }
            }
            b'=' => {
                if self.peek_byte() == Some(b'=') {
                    self.advance();
                    Token::Eq2
                } else {
                    Token::Eq
                }
            }
            _ => {
                return Err(ParseError {
                    file: self.file.to_string(),
                    line,
                    message: format!("unexpected character: '{}'", b as char),
                });
            }
        };

        Ok(Located { token: tok, line })
    }

    fn lex_string(&mut self, line: usize) -> Result<Located, ParseError> {
        self.advance(); // consume opening "
        let start = self.pos;
        loop {
            match self.advance() {
                Some(b'"') => {
                    let s = std::str::from_utf8(&self.src[start..self.pos - 1])
                        .map_err(|_| self.err("invalid UTF-8 in string literal"))?;
                    return Ok(Located {
                        token: Token::StringLit(s.to_string()),
                        line,
                    });
                }
                Some(b'\n') | None => {
                    return Err(ParseError {
                        file: self.file.to_string(),
                        line,
                        message: "unterminated string literal".into(),
                    });
                }
                _ => {}
            }
        }
    }

    fn lex_number(&mut self, line: usize) -> Result<Located, ParseError> {
        if self.peek_byte() == Some(b'0') && self.pos + 1 < self.src.len() {
            let next = self.src[self.pos + 1];
            if next == b'x' || next == b'X' {
                self.advance(); // '0'
                self.advance(); // 'x'
                let start = self.pos;
                while let Some(b) = self.peek_byte() {
                    if b.is_ascii_hexdigit() {
                        self.advance();
                    } else {
                        break;
                    }
                }
                if self.pos == start {
                    return Err(ParseError {
                        file: self.file.to_string(),
                        line,
                        message: "expected hex digits after '0x'".into(),
                    });
                }
                let hex_str = std::str::from_utf8(&self.src[start..self.pos])
                    .map_err(|_| self.err("invalid UTF-8 in hex literal"))?;
                let val = u64::from_str_radix(hex_str, 16).map_err(|e| ParseError {
                    file: self.file.to_string(),
                    line,
                    message: format!("invalid hex literal: {}", e),
                })?;
                return Ok(Located {
                    token: Token::HexLit(val),
                    line,
                });
            }
        }

        let start = self.pos;
        while let Some(b) = self.peek_byte() {
            if b.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }

        // CRITICAL: reject float literals — if next char is '.', error
        if let Some(b'.') = self.peek_byte() {
            // Check it's followed by a digit (actual float, not member access after number)
            if self.pos + 1 < self.src.len() && self.src[self.pos + 1].is_ascii_digit() {
                return Err(ParseError {
                    file: self.file.to_string(),
                    line,
                    message: "float literals are not allowed — VixiScript uses integers only"
                        .into(),
                });
            }
        }

        let num_str = std::str::from_utf8(&self.src[start..self.pos])
            .map_err(|_| self.err("invalid UTF-8 in number"))?;
        let val: u64 = num_str.parse().map_err(|e| ParseError {
            file: self.file.to_string(),
            line,
            message: format!("invalid integer literal: {}", e),
        })?;

        // Check for 'p' suffix → Permyriad literal (e.g. 5000p = 50%)
        if self.peek_byte() == Some(b'p') {
            self.advance(); // consume 'p'
            let clamped = (val as i64).clamp(-10000, 10000) as i32;
            return Ok(Located {
                token: Token::PermyriadLit(clamped),
                line,
            });
        }

        Ok(Located {
            token: Token::IntLit(val),
            line,
        })
    }

    fn lex_ident(&mut self, line: usize) -> Result<Located, ParseError> {
        let start = self.pos;
        while let Some(b) = self.peek_byte() {
            if b.is_ascii_alphanumeric() || b == b'_' {
                self.advance();
            } else {
                break;
            }
        }
        let s = std::str::from_utf8(&self.src[start..self.pos])
            .map_err(|_| self.err("invalid UTF-8 in identifier"))?;
        Ok(Located {
            token: Token::Ident(s.to_string()),
            line,
        })
    }
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Located,
    file: String,
    material_counter: u16,
    spatial_counter: u16,
    automata_counter: u16,
    ui_counter: u16,
    atom_counter: u16,
    acrylic_counter: u16,
    pressure_counter: u16,
    layers_counter: u16,
    viewport_counter: u16,
    brush_counter: u16,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str, file_name: &'a str) -> Self {
        let mut lexer = Lexer::new(source, file_name);
        // Prime the first token — if lexing fails, store Eof and we'll error on first consume.
        let current = lexer.next_token().unwrap_or(Located {
            token: Token::Eof,
            line: 1,
        });
        Self {
            lexer,
            current,
            file: file_name.to_string(),
            material_counter: 0,
            spatial_counter: 0,
            automata_counter: 0,
            ui_counter: 0,
            atom_counter: 0,
            acrylic_counter: 0,
            pressure_counter: 0,
            layers_counter: 0,
            viewport_counter: 0,
            brush_counter: 0,
        }
    }

    // -- Token helpers -------------------------------------------------------

    fn peek(&self) -> &Token {
        &self.current.token
    }

    fn current_line(&self) -> usize {
        self.current.line
    }

    fn advance(&mut self) -> Result<Located, ParseError> {
        let prev = self.current.clone();
        self.current = self.lexer.next_token()?;
        Ok(prev)
    }

    fn expect_ident(&mut self, expected: &str) -> Result<(), ParseError> {
        match &self.current.token {
            Token::Ident(s) if s == expected => {
                self.advance()?;
                Ok(())
            }
            other => Err(ParseError {
                file: self.file.clone(),
                line: self.current.line,
                message: format!("expected '{}', got {:?}", expected, other),
            }),
        }
    }

    fn expect(&mut self, expected: &Token) -> Result<Located, ParseError> {
        if &self.current.token == expected {
            self.advance()
        } else {
            Err(ParseError {
                file: self.file.clone(),
                line: self.current.line,
                message: format!("expected {:?}, got {:?}", expected, self.current.token),
            })
        }
    }

    fn err(&self, msg: impl Into<String>) -> ParseError {
        ParseError {
            file: self.file.clone(),
            line: self.current.line,
            message: msg.into(),
        }
    }

    // -- Top-level parse -----------------------------------------------------

    fn parse_source(&mut self) -> Result<VixelAst, ParseError> {
        let mut ast = VixelAst::new();

        loop {
            match self.peek() {
                Token::Eof => break,
                Token::Ident(s) => {
                    let keyword = s.clone();
                    match keyword.as_str() {
                        "material" => {
                            let mat = self.parse_material()?;
                            ast.materials.push(mat);
                        }
                        "ui" => {
                            let ui = self.parse_ui()?;
                            ast.ui_defs.push(ui);
                        }
                        "theme" => {
                            let theme = self.parse_theme()?;
                            ast.themes.push(theme);
                        }
                        "atom" => {
                            let atom = self.parse_atom()?;
                            ast.atoms.push(atom);
                        }
                        "acrylic" => {
                            let a = self.parse_acrylic()?;
                            ast.acrylics.push(a);
                        }
                        "pressure" => {
                            let p = self.parse_pressure()?;
                            ast.pressures.push(p);
                        }
                        "layers" => {
                            let l = self.parse_layers()?;
                            ast.layers.push(l);
                        }
                        "viewport" => {
                            let v = self.parse_viewport()?;
                            ast.viewports.push(v);
                        }
                        "brush" => {
                            let b = self.parse_brush()?;
                            ast.brushes.push(b);
                        }
                        "rule" => {
                            let rule = self.parse_rule()?;
                            ast.automata.push(rule);
                        }
                        s if s.starts_with("spawn_") => {
                            let spatial = self.parse_spawn()?;
                            ast.spatials.push(spatial);
                        }
                        s if s.starts_with("set_") => {
                            let env = self.parse_set()?;
                            ast.environment.push(env);
                        }
                        _ => {
                            return Err(self.err(format!(
                                "unexpected top-level keyword: '{}'",
                                keyword
                            )));
                        }
                    }
                }
                other => {
                    return Err(self.err(format!("unexpected token at top level: {:?}", other)));
                }
            }
        }

        Ok(ast)
    }

    // -- Material block ------------------------------------------------------

    fn parse_material(&mut self) -> Result<MaterialDef, ParseError> {
        self.expect_ident("material")?;

        // material "name" { ... }
        let name = self.expect_string_lit()?;
        self.expect(&Token::LBrace)?;

        let mut mat = MaterialDef::default();
        mat.id = self.material_counter;
        self.material_counter += 1;

        let name_bytes = name.as_bytes();
        let copy_len = name_bytes.len().min(32);
        mat.name[..copy_len].copy_from_slice(&name_bytes[..copy_len]);
        mat.name_len = copy_len;

        // Parse properties until '}'
        loop {
            if *self.peek() == Token::RBrace {
                self.advance()?;
                break;
            }

            let prop_name = self.expect_any_ident()?;
            self.expect(&Token::Colon)?;

            match prop_name.as_str() {
                "mass" => mat.mass_pmy = self.expect_u16_value()?,
                "hardness" => mat.hardness_pmy = self.expect_u16_value()?,
                "flammability" => mat.flammability_pmy = self.expect_u16_value()?,
                "roughness" => mat.roughness_pmy = self.expect_u16_value()?,
                "metallic" => mat.metallic_pmy = self.expect_u16_value()?,
                "albedo" => mat.albedo = self.expect_u32_value()?,
                "destruction" => {
                    let mode_str = self.expect_string_lit()?;
                    mat.destruction_mode = match mode_str.as_str() {
                        "shatter" => 0,
                        "splinter" => 1,
                        "melt" => 2,
                        _ => {
                            return Err(self.err(format!(
                                "unknown destruction mode: '{}' (expected shatter, splinter, or melt)",
                                mode_str
                            )));
                        }
                    };
                }
                _ => {
                    return Err(self.err(format!(
                        "unknown material property: '{}'",
                        prop_name
                    )));
                }
            }

            // Optional trailing comma
            if *self.peek() == Token::Comma {
                self.advance()?;
            }
        }

        Ok(mat)
    }

    // -- Atom block ----------------------------------------------------------
    // `atom { coord: (x, y), material_id: N, resonance: Np, color: 0xRRGGBB }`
    // VixiScript's lowering target — authors a VixelAtom
    // (forge_daemon_types::atom::VixelAtom: ColourID/MaterialID/Resonance).
    fn parse_atom(&mut self) -> Result<AtomDef, ParseError> {
        self.expect_ident("atom")?;
        self.expect(&Token::LBrace)?;

        let mut atom = AtomDef { id: self.atom_counter, ..Default::default() };
        self.atom_counter += 1;

        loop {
            if *self.peek() == Token::RBrace {
                self.advance()?;
                break;
            }

            let key = self.expect_any_ident()?;
            self.expect(&Token::Colon)?;

            match key.as_str() {
                "coord" => {
                    self.expect(&Token::LParen)?;
                    let x = self.expect_int_lit()? as i32;
                    self.expect(&Token::Comma)?;
                    let y = self.expect_int_lit()? as i32;
                    self.expect(&Token::RParen)?;
                    atom.coord = (x, y);
                }
                "material_id" => atom.material_id = self.expect_u16_value()?,
                "resonance" => atom.resonance = self.expect_u16_value()?,
                "color" => atom.color = self.expect_u32_value()?,
                _ => {
                    return Err(self.err(format!("unknown atom property: '{}'", key)));
                }
            }

            if *self.peek() == Token::Comma {
                self.advance()?;
            }
        }

        Ok(atom)
    }

    // -- Acrylic block (paint dab) -------------------------------------------
    // `acrylic { color: 0xRRGGBB, material_id: N, essence_id: N, phase: Np }`
    // Authors forge_core::acrylic::AcrylicLoad / stamp_acrylic.
    fn parse_acrylic(&mut self) -> Result<AcrylicDef, ParseError> {
        self.expect_ident("acrylic")?;
        self.expect(&Token::LBrace)?;

        let mut a = AcrylicDef { id: self.acrylic_counter, ..Default::default() };
        self.acrylic_counter += 1;

        loop {
            if *self.peek() == Token::RBrace {
                self.advance()?;
                break;
            }

            let key = self.expect_any_ident()?;
            self.expect(&Token::Colon)?;

            match key.as_str() {
                "color" => a.color = self.expect_u32_value()?,
                "material_id" => a.material_id = self.expect_u16_value()?,
                "essence_id" => a.essence_id = self.expect_u16_value()?,
                "phase" => a.phase = self.expect_u16_value()?,
                _ => {
                    return Err(self.err(format!("unknown acrylic property: '{}'", key)));
                }
            }

            if *self.peek() == Token::Comma {
                self.advance()?;
            }
        }

        Ok(a)
    }

    // -- Pressure block (pen-feel curve) -------------------------------------
    // `pressure { curve: linear|soft|hard }`
    // Authors forge_core::pressure::PressureCurve.
    fn parse_pressure(&mut self) -> Result<PressureDef, ParseError> {
        self.expect_ident("pressure")?;
        self.expect(&Token::LBrace)?;

        let mut p = PressureDef { id: self.pressure_counter, ..Default::default() };
        self.pressure_counter += 1;

        loop {
            if *self.peek() == Token::RBrace {
                self.advance()?;
                break;
            }

            let key = self.expect_any_ident()?;
            self.expect(&Token::Colon)?;

            match key.as_str() {
                "curve" => {
                    let name = self.expect_any_ident()?;
                    p.curve = match name.as_str() {
                        "linear" => 0,
                        "soft" => 1,
                        "hard" => 2,
                        _ => {
                            return Err(self.err(format!(
                                "unknown pressure curve: '{}' (expected linear, soft, or hard)",
                                name
                            )));
                        }
                    };
                }
                _ => {
                    return Err(self.err(format!("unknown pressure property: '{}'", key)));
                }
            }

            if *self.peek() == Token::Comma {
                self.advance()?;
            }
        }

        Ok(p)
    }

    // -- Layers block (paint-layer stack depth) ------------------------------
    // `layers { count: N }` — authors forge_core::layer_stack::LayerStack.
    fn parse_layers(&mut self) -> Result<LayersDef, ParseError> {
        self.expect_ident("layers")?;
        self.expect(&Token::LBrace)?;

        let mut l = LayersDef { id: self.layers_counter, ..Default::default() };
        self.layers_counter += 1;

        loop {
            if *self.peek() == Token::RBrace {
                self.advance()?;
                break;
            }
            let key = self.expect_any_ident()?;
            self.expect(&Token::Colon)?;
            match key.as_str() {
                "count" => l.count = self.expect_u16_value()?,
                _ => {
                    return Err(self.err(format!("unknown layers property: '{}'", key)));
                }
            }
            if *self.peek() == Token::Comma {
                self.advance()?;
            }
        }

        Ok(l)
    }

    // -- Viewport block (camera uniform) -------------------------------------
    // `viewport { w: N, h: N, zoom: Np }` — authors forge_gpu::vixel_pass::VixelViewport.
    fn parse_viewport(&mut self) -> Result<ViewportDef, ParseError> {
        self.expect_ident("viewport")?;
        self.expect(&Token::LBrace)?;

        let mut v = ViewportDef { id: self.viewport_counter, ..Default::default() };
        self.viewport_counter += 1;

        loop {
            if *self.peek() == Token::RBrace {
                self.advance()?;
                break;
            }
            let key = self.expect_any_ident()?;
            self.expect(&Token::Colon)?;
            match key.as_str() {
                "w" => v.w = self.expect_u16_value()?,
                "h" => v.h = self.expect_u16_value()?,
                "zoom" => v.zoom = self.expect_u16_value()?,
                _ => {
                    return Err(self.err(format!("unknown viewport property: '{}'", key)));
                }
            }
            if *self.peek() == Token::Comma {
                self.advance()?;
            }
        }

        Ok(v)
    }

    // -- Brush block (procedural brush tip) ----------------------------------
    // `brush { w: N, h: N, falloff: Np }` — authors forge_core::brush_mask::BrushMask / MaskStamp.
    fn parse_brush(&mut self) -> Result<BrushDef, ParseError> {
        self.expect_ident("brush")?;
        self.expect(&Token::LBrace)?;

        let mut b = BrushDef { id: self.brush_counter, ..Default::default() };
        self.brush_counter += 1;

        loop {
            if *self.peek() == Token::RBrace {
                self.advance()?;
                break;
            }
            let key = self.expect_any_ident()?;
            self.expect(&Token::Colon)?;
            match key.as_str() {
                "w" => b.w = self.expect_u16_value()?,
                "h" => b.h = self.expect_u16_value()?,
                "falloff" => b.falloff = self.expect_u16_value()?,
                _ => {
                    return Err(self.err(format!("unknown brush property: '{}'", key)));
                }
            }
            if *self.peek() == Token::Comma {
                self.advance()?;
            }
        }

        Ok(b)
    }

    // -- Theme block ------------------------------------------------------------

    fn parse_theme(&mut self) -> Result<ThemeDef, ParseError> {
        self.expect_ident("theme")?;
        let name = self.expect_string_lit()?;
        self.expect(&Token::LBrace)?;

        let mut theme = ThemeDef {
            name,
            layer: ThemeLayer::Base,
            tokens: Vec::new(),
        };

        loop {
            if *self.peek() == Token::RBrace {
                self.advance()?;
                break;
            }

            let key = self.expect_any_ident()?;
            self.expect(&Token::Colon)?;

            match key.as_str() {
                "token" => {
                    // token accent_creation: 0xF0A840FF
                    let tok_name = self.expect_any_ident()?;
                    self.expect(&Token::Colon)?;
                    let value = self.expect_u32_value()?;
                    theme.tokens.push(TokenDef { name: tok_name, value });
                }
                "layer" => {
                    let layer_name = self.expect_any_ident()?;
                    theme.layer = match layer_name.as_str() {
                        "base" => ThemeLayer::Base,
                        "profile" => ThemeLayer::Profile,
                        "celestial" => ThemeLayer::Celestial,
                        "override" => ThemeLayer::Override,
                        _ => return Err(self.err(format!("unknown layer: '{}'", layer_name))),
                    };
                }
                _ => return Err(self.err(format!("unknown theme key: '{}'", key))),
            }

            if *self.peek() == Token::Comma {
                self.advance()?;
            }
        }

        Ok(theme)
    }

    // -- UI block ------------------------------------------------------------

    fn parse_ui(&mut self) -> Result<UiDef, ParseError> {
        self.expect_ident("ui")?;

        let name = self.expect_string_lit()?;
        self.expect(&Token::LBrace)?;

        let mut ui = UiDef::default();
        ui.id = self.ui_counter;
        self.ui_counter += 1;

        let name_bytes = name.as_bytes();
        let copy_len = name_bytes.len().min(32);
        ui.name[..copy_len].copy_from_slice(&name_bytes[..copy_len]);
        ui.name_len = copy_len;

        loop {
            if *self.peek() == Token::RBrace {
                self.advance()?;
                break;
            }

            let prop_name = self.expect_any_ident()?;
            self.expect(&Token::Colon)?;

            match prop_name.as_str() {
                // Original fields
                "x" => ui.x = self.expect_int_lit()? as i64,
                "y" => ui.y = self.expect_int_lit()? as i64,
                "w" => ui.w = self.expect_int_lit()? as i64,
                "h" => ui.h = self.expect_int_lit()? as i64,
                "radius" => ui.radius = self.expect_u16_value()?,
                "vibe" => ui.vibe_mask = self.expect_u16_value()? as u8,

                // Color — literal or $token
                "color" => ui.color = self.parse_color_value()?,
                "color_selected" => ui.color_selected = Some(self.parse_color_value()?),

                // Material — by name or index
                "material" => {
                    match self.peek() {
                        Token::Ident(_) => {
                            ui.material_name = Some(self.expect_any_ident()?);
                        }
                        _ => ui.material_idx = self.expect_u16_value()? as u8,
                    }
                }

                // Extended flat fields
                "depth" => ui.depth = self.expect_int_lit()? as i64,
                "font" => ui.font = self.expect_u16_value()?,
                "parent" => ui.parent = Some(self.expect_string_lit()?),
                "spacing" => ui.spacing = self.expect_int_lit()? as i64,
                "repeat" => {
                    let dir = self.expect_any_ident()?;
                    ui.repeat = match dir.as_str() {
                        "vertical" => RepeatDir::Vertical,
                        "horizontal" => RepeatDir::Horizontal,
                        _ => return Err(self.err(format!("unknown repeat: '{}'", dir))),
                    };
                }

                // Sound events
                "sound_show" => ui.sound_show = Some(self.expect_string_lit()?),
                "sound_dismiss" => ui.sound_dismiss = Some(self.expect_string_lit()?),
                "sound_hover" => ui.sound_hover = Some(self.expect_string_lit()?),
                "sound_select" => ui.sound_select = Some(self.expect_string_lit()?),

                // Text content
                "text" => ui.text = Some(self.expect_string_lit()?),
                "text_color" => {
                    let c = self.parse_color_value()?;
                    if let ColorValue::Literal(v) = c { ui.text_color = Some(v); }
                }
                "font_size" => ui.font_size = self.expect_u16_value()?,

                // Voxel text (3D font)
                "voxel_text" => ui.voxel_text = Some(self.expect_string_lit()?),
                "voxel_material" => ui.voxel_material = Some(self.expect_any_ident()?),

                // Nested sub-blocks
                "spring_in" => ui.spring_in = Some(self.parse_spring_block()?),
                "spring_hover" => ui.spring_hover = Some(self.parse_spring_block()?),
                "particle" => ui.particle = Some(self.parse_particle_block()?),
                "particle_selected" => ui.particle_selected = Some(self.parse_particle_block()?),
                "fill" => ui.fill = Some(self.parse_fill_block()?),

                // Inline conditional rules
                other if other.starts_with("rule_") => {
                    ui.rules.push(self.parse_conditional_rule()?);
                }

                _ => {
                    return Err(self.err(format!(
                        "unknown ui property: '{}'",
                        prop_name
                    )));
                }
            }

            if *self.peek() == Token::Comma {
                self.advance()?;
            }
        }

        Ok(ui)
    }

    // -- UI sub-block helpers ------------------------------------------------

    fn parse_color_value(&mut self) -> Result<ColorValue, ParseError> {
        match self.peek() {
            Token::TokenRef(_) => {
                if let Token::TokenRef(name) = &self.current.token {
                    let n = name.clone();
                    self.advance()?;
                    Ok(ColorValue::TokenRef(n))
                } else {
                    unreachable!()
                }
            }
            Token::HexLit(_) | Token::IntLit(_) | Token::PermyriadLit(_) => {
                let v = self.expect_int_lit()? as u32;
                Ok(ColorValue::Literal(v))
            }
            other => Err(self.err(format!("expected color literal or $token, got {:?}", other))),
        }
    }

    fn parse_spring_block(&mut self) -> Result<SpringDef, ParseError> {
        self.expect(&Token::LBrace)?;
        let mut spring = SpringDef::default();
        loop {
            if *self.peek() == Token::RBrace { self.advance()?; break; }
            let key = self.expect_any_ident()?;
            self.expect(&Token::Colon)?;
            match key.as_str() {
                "stiffness" => spring.stiffness = self.expect_int_lit()? as i32,
                "damping" => spring.damping = self.expect_int_lit()? as i32,
                "scale" => spring.scale = self.expect_int_lit()? as i32,
                _ => return Err(self.err(format!("unknown spring key: '{}'", key))),
            }
            if *self.peek() == Token::Comma { self.advance()?; }
        }
        Ok(spring)
    }

    fn parse_particle_block(&mut self) -> Result<ParticleDef, ParseError> {
        self.expect(&Token::LBrace)?;
        let mut p = ParticleDef::default();
        loop {
            if *self.peek() == Token::RBrace { self.advance()?; break; }
            let key = self.expect_any_ident()?;
            self.expect(&Token::Colon)?;
            match key.as_str() {
                "emitter" => {
                    let e = self.expect_any_ident()?;
                    p.emitter = match e.as_str() {
                        "edge" => EmitterType::Edge,
                        "fill" => EmitterType::Fill,
                        "point" => EmitterType::Point,
                        _ => return Err(self.err(format!("unknown emitter: '{}'", e))),
                    };
                }
                "color" => p.color = self.parse_color_value()?,
                "rate" => p.rate = self.expect_u16_value()?,
                "lifetime" => p.lifetime = self.expect_u16_value()?,
                "intensity_bind" => {
                    p.intensity_bind = Some(BindTarget(self.expect_any_ident()?));
                }
                _ => return Err(self.err(format!("unknown particle key: '{}'", key))),
            }
            if *self.peek() == Token::Comma { self.advance()?; }
        }
        Ok(p)
    }

    fn parse_fill_block(&mut self) -> Result<FillDef, ParseError> {
        self.expect(&Token::LBrace)?;
        let mut f = FillDef::default();
        loop {
            if *self.peek() == Token::RBrace { self.advance()?; break; }
            let key = self.expect_any_ident()?;
            self.expect(&Token::Colon)?;
            match key.as_str() {
                "material" => f.material_name = self.expect_any_ident()?,
                "bind" => f.bind = BindTarget(self.expect_any_ident()?),
                "direction" => {
                    let d = self.expect_any_ident()?;
                    f.direction = match d.as_str() {
                        "horizontal" => FillDir::Horizontal,
                        "vertical" => FillDir::Vertical,
                        _ => return Err(self.err(format!("unknown direction: '{}'", d))),
                    };
                }
                _ => return Err(self.err(format!("unknown fill key: '{}'", key))),
            }
            if *self.peek() == Token::Comma { self.advance()?; }
        }
        Ok(f)
    }

    fn parse_conditional_rule(&mut self) -> Result<ConditionalRule, ParseError> {
        self.expect(&Token::LBrace)?;
        let mut rule = ConditionalRule {
            bind: BindTarget::default(),
            op: CmpOp::Lt,
            threshold: 0,
            color_override: None,
            emissive_override: None,
            particle: None,
            sound: None,
        };
        loop {
            if *self.peek() == Token::RBrace { self.advance()?; break; }
            let key = self.expect_any_ident()?;
            self.expect(&Token::Colon)?;
            match key.as_str() {
                "when" => {
                    let bind_name = self.expect_any_ident()?;
                    rule.bind = BindTarget(bind_name);
                    match self.peek() {
                        Token::Lt => { self.advance()?; rule.op = CmpOp::Lt; }
                        Token::Gt => { self.advance()?; rule.op = CmpOp::Gt; }
                        _ => return Err(self.err("expected < or > in condition")),
                    }
                    rule.threshold = self.expect_int_lit()? as i32;
                }
                "color" => rule.color_override = Some(self.parse_color_value()?),
                "emissive" => rule.emissive_override = Some(self.expect_int_lit()? as i32),
                "particle" => rule.particle = Some(self.parse_particle_block()?),
                "sound" => rule.sound = Some(self.expect_string_lit()?),
                _ => return Err(self.err(format!("unknown rule key: '{}'", key))),
            }
            if *self.peek() == Token::Comma { self.advance()?; }
        }
        Ok(rule)
    }

    // -- Rule block ----------------------------------------------------------

    fn parse_rule(&mut self) -> Result<AutomataDef, ParseError> {
        self.expect_ident("rule")?;

        let name = self.expect_string_lit()?;
        self.expect(&Token::LBrace)?;

        let mut when_text = String::new();
        let mut then_text = String::new();
        let mut _tick_delay: u64 = 1;

        // Parse fields: when, then, tick_delay
        loop {
            if *self.peek() == Token::RBrace {
                self.advance()?;
                break;
            }

            let field = self.expect_any_ident()?;
            self.expect(&Token::Colon)?;

            match field.as_str() {
                "when" => {
                    when_text = self.collect_expression_text()?;
                }
                "then" => {
                    then_text = self.collect_expression_text()?;
                }
                "tick_delay" => {
                    _tick_delay = self.expect_int_lit()?;
                    // Optional trailing comma
                    if *self.peek() == Token::Comma {
                        self.advance()?;
                    }
                }
                _ => {
                    return Err(self.err(format!("unknown rule field: '{}'", field)));
                }
            }
        }

        // Build combined wgsl_source from when + then
        let wgsl_source = format!("when: {} then: {}", when_text, then_text);

        // Infer automata type from name and content
        let combined = format!("{} {} {}", name, when_text, then_text).to_lowercase();
        let rule_type = infer_automata_type(&combined);

        let mut def = AutomataDef::default();
        def.id = self.automata_counter;
        self.automata_counter += 1;
        def.rule_type = rule_type;
        def.wgsl_source = wgsl_source;

        Ok(def)
    }

    /// Collect tokens as text until we hit a comma at the field boundary
    /// (i.e., comma not inside parens/brackets).
    fn collect_expression_text(&mut self) -> Result<String, ParseError> {
        let mut parts: Vec<String> = Vec::new();
        let mut depth: i32 = 0; // paren + bracket nesting

        loop {
            match self.peek() {
                Token::RBrace if depth == 0 => break,
                Token::Comma if depth == 0 => {
                    self.advance()?; // consume the field-separating comma
                    break;
                }
                Token::Eof => {
                    return Err(self.err("unexpected end of file in expression"));
                }
                _ => {}
            }

            let tok = self.advance()?;
            match &tok.token {
                Token::LParen | Token::LBracket => depth += 1,
                Token::RParen | Token::RBracket => depth -= 1,
                _ => {}
            }
            parts.push(token_to_string(&tok.token));
        }

        Ok(parts.join(" "))
    }

    // -- Spawn calls ---------------------------------------------------------

    fn parse_spawn(&mut self) -> Result<SpatialDef, ParseError> {
        let fn_name = self.expect_any_ident()?;

        // Validate spawn function name
        match fn_name.as_str() {
            "spawn_grid" | "spawn_forest" | "spawn_line" | "spawn_sphere" | "spawn_scatter" => {}
            _ => {
                return Err(self.err(format!("unknown spawn function: '{}'", fn_name)));
            }
        }

        self.expect(&Token::LParen)?;

        let mut def = SpatialDef::default();
        def.id = self.spatial_counter;
        self.spatial_counter += 1;

        // Parse arguments (positional and named)
        let mut socket_idx: usize = 0;
        let mut first = true;

        loop {
            if *self.peek() == Token::RParen {
                self.advance()?;
                break;
            }

            if !first {
                self.expect(&Token::Comma)?;
            }
            first = false;

            // Check for named argument: `name: value`
            if let Token::Ident(name) = self.peek().clone() {
                // Peek ahead to see if next is ':'
                let saved_line = self.current_line();
                let saved_tok = self.current.clone();
                self.advance()?;

                if *self.peek() == Token::Colon {
                    self.advance()?; // consume ':'
                    // Named argument — parse value
                    match name.as_str() {
                        "radius" | "strength" => {
                            let val = self.expect_int_lit()?;
                            def.stress_limit_pmy = val as u16;
                        }
                        "from" | "to" => {
                            let arr = self.parse_array_literal()?;
                            if socket_idx < 6 && arr.len() >= 3 {
                                def.sockets[socket_idx] =
                                    (arr[0] as i8, arr[1] as i8, arr[2] as i8);
                                def.socket_count += 1;
                                socket_idx += 1;
                            }
                        }
                        _ => {
                            // Skip unknown named args — just parse the value
                            self.skip_value()?;
                        }
                    }
                    continue;
                } else {
                    // Not a named arg — restore and parse as positional
                    // We already consumed the ident, so we need to handle it
                    // The ident was a string-like positional arg (material name reference)
                    // Actually this was an ident not followed by ':', so it's a positional value.
                    // We already advanced past it. The current token is whatever came after.
                    // For spawn calls, positional idents aren't expected — only strings and ints.
                    // But we consumed it, so just continue.
                    let _ = saved_line;
                    let _ = saved_tok;
                    continue;
                }
            }

            // Positional argument: string, int, hex, or array
            match self.peek() {
                Token::StringLit(_) => {
                    self.advance()?; // material name — already captured in spatial by convention
                }
                Token::IntLit(_) | Token::PermyriadLit(_) | Token::HexLit(_) => {
                    self.advance()?; // positional int (grid dims, count, etc.)
                }
                Token::LBracket => {
                    let arr = self.parse_array_literal()?;
                    if socket_idx < 6 && arr.len() >= 3 {
                        def.sockets[socket_idx] = (arr[0] as i8, arr[1] as i8, arr[2] as i8);
                        def.socket_count += 1;
                        socket_idx += 1;
                    }
                }
                _ => {
                    return Err(self.err(format!(
                        "unexpected token in spawn arguments: {:?}",
                        self.peek()
                    )));
                }
            }
        }

        self.expect(&Token::Semicolon)?;
        Ok(def)
    }

    // -- Set calls -----------------------------------------------------------

    fn parse_set(&mut self) -> Result<EnvironmentDef, ParseError> {
        let fn_name = self.expect_any_ident()?;

        let env_type = match fn_name.as_str() {
            "set_temperature" => EnvironmentType::Temperature,
            "set_wind" => EnvironmentType::Wind,
            "set_gravity" => EnvironmentType::Gravity,
            _ => {
                return Err(self.err(format!("unknown set function: '{}'", fn_name)));
            }
        };

        self.expect(&Token::LParen)?;

        let mut def = EnvironmentDef::default();
        def.env_type = env_type;

        // Parse arguments
        let mut first = true;
        loop {
            if *self.peek() == Token::RParen {
                self.advance()?;
                break;
            }

            if !first {
                self.expect(&Token::Comma)?;
            }
            first = false;

            // Check for named argument
            if let Token::Ident(name) = self.peek().clone() {
                let saved = self.current.clone();
                self.advance()?;

                if *self.peek() == Token::Colon {
                    self.advance()?;
                    match name.as_str() {
                        "strength" => {
                            def.value_pmy = self.expect_u16_value()?;
                        }
                        _ => {
                            self.skip_value()?;
                        }
                    }
                    continue;
                } else {
                    // Not named — was a bare ident, ignore
                    let _ = saved;
                    continue;
                }
            }

            // Positional: string (target), int (value), or array (vector)
            match self.peek() {
                Token::StringLit(_) => {
                    let s = self.expect_string_lit()?;
                    let s_bytes = s.as_bytes();
                    let copy_len = s_bytes.len().min(32);
                    def.target[..copy_len].copy_from_slice(&s_bytes[..copy_len]);
                    def.target_len = copy_len;
                }
                Token::IntLit(_) | Token::PermyriadLit(_) | Token::HexLit(_) => {
                    def.value_pmy = self.expect_u16_value()?;
                }
                Token::LBracket => {
                    let arr = self.parse_array_literal()?;
                    if arr.len() >= 3 {
                        def.vector = [arr[0] as i32, arr[1] as i32, arr[2] as i32];
                    }
                }
                _ => {
                    return Err(self.err(format!(
                        "unexpected token in set arguments: {:?}",
                        self.peek()
                    )));
                }
            }
        }

        self.expect(&Token::Semicolon)?;
        Ok(def)
    }

    // -- Helpers -------------------------------------------------------------

    fn expect_string_lit(&mut self) -> Result<String, ParseError> {
        match &self.current.token {
            Token::StringLit(s) => {
                let s = s.clone();
                self.advance()?;
                Ok(s)
            }
            other => Err(self.err(format!("expected string literal, got {:?}", other))),
        }
    }

    fn expect_any_ident(&mut self) -> Result<String, ParseError> {
        match &self.current.token {
            Token::Ident(s) => {
                let s = s.clone();
                self.advance()?;
                Ok(s)
            }
            other => Err(self.err(format!("expected identifier, got {:?}", other))),
        }
    }

    fn expect_int_lit(&mut self) -> Result<u64, ParseError> {
        match &self.current.token {
            Token::IntLit(v) => {
                let v = *v;
                self.advance()?;
                Ok(v)
            }
            Token::PermyriadLit(v) => {
                let v = *v as u64;
                self.advance()?;
                Ok(v)
            }
            Token::HexLit(v) => {
                let v = *v;
                self.advance()?;
                Ok(v)
            }
            other => Err(self.err(format!("expected integer literal, got {:?}", other))),
        }
    }

    fn expect_u16_value(&mut self) -> Result<u16, ParseError> {
        let v = self.expect_int_lit()?;
        Ok(v as u16)
    }

    fn expect_u32_value(&mut self) -> Result<u32, ParseError> {
        let v = self.expect_int_lit()?;
        Ok(v as u32)
    }

    fn parse_array_literal(&mut self) -> Result<Vec<i64>, ParseError> {
        self.expect(&Token::LBracket)?;
        let mut values = Vec::new();
        let mut first = true;

        loop {
            if *self.peek() == Token::RBracket {
                self.advance()?;
                break;
            }

            if !first {
                self.expect(&Token::Comma)?;
            }
            first = false;

            let val = self.parse_array_element()?;
            values.push(val);
        }

        Ok(values)
    }

    fn parse_array_element(&mut self) -> Result<i64, ParseError> {
        // Handle unary minus for negative array elements like [0, -1, 0]
        let negate = if *self.peek() == Token::Minus {
            self.advance()?;
            true
        } else {
            false
        };

        match &self.current.token {
            Token::IntLit(v) => {
                let v = *v as i64;
                self.advance()?;
                Ok(if negate { -v } else { v })
            }
            Token::PermyriadLit(v) => {
                let v = *v as i64;
                self.advance()?;
                Ok(if negate { -v } else { v })
            }
            Token::HexLit(v) => {
                let v = *v as i64;
                self.advance()?;
                Ok(if negate { -v } else { v })
            }
            other => Err(self.err(format!("expected integer in array, got {:?}", other))),
        }
    }

    fn skip_value(&mut self) -> Result<(), ParseError> {
        // Skip a single value (int, string, array, or ident)
        match self.peek() {
            Token::IntLit(_) | Token::PermyriadLit(_) | Token::HexLit(_) | Token::StringLit(_) | Token::Ident(_) => {
                self.advance()?;
                Ok(())
            }
            Token::LBracket => {
                self.parse_array_literal()?;
                Ok(())
            }
            _ => Err(self.err(format!("unexpected token when skipping value: {:?}", self.peek()))),
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers (free functions)
// ---------------------------------------------------------------------------

/// Infer automata type from combined rule name + when/then content.
fn infer_automata_type(combined: &str) -> AutomataType {
    if combined.contains("fire") || combined.contains("flammab") {
        AutomataType::Fire
    } else if combined.contains("fluid") || combined.contains("water") {
        AutomataType::Fluid
    } else if combined.contains("gravity") || combined.contains("below") {
        AutomataType::Gravity
    } else {
        AutomataType::Custom
    }
}

/// Convert a token back to its string representation (for expression collection).
fn token_to_string(tok: &Token) -> String {
    match tok {
        Token::Ident(s) => s.clone(),
        Token::StringLit(s) => format!("\"{}\"", s),
        Token::IntLit(v) => v.to_string(),
        Token::PermyriadLit(v) => format!("{}p", v),
        Token::HexLit(v) => format!("0x{:X}", v),
        Token::LBrace => "{".into(),
        Token::RBrace => "}".into(),
        Token::LParen => "(".into(),
        Token::RParen => ")".into(),
        Token::LBracket => "[".into(),
        Token::RBracket => "]".into(),
        Token::Colon => ":".into(),
        Token::Comma => ",".into(),
        Token::Semicolon => ";".into(),
        Token::Dot => ".".into(),
        Token::Gt => ">".into(),
        Token::Lt => "<".into(),
        Token::Ampersand2 => "&&".into(),
        Token::Pipe2 => "||".into(),
        Token::Bang => "!".into(),
        Token::Eq => "=".into(),
        Token::Eq2 => "==".into(),
        Token::Minus => "-".into(),
        Token::Plus => "+".into(),
        Token::TokenRef(s) => format!("${}", s),
        Token::Eof => "".into(),
    }
}

// ---------------------------------------------------------------------------
// Hex-prism lowering
// ---------------------------------------------------------------------------

/// Lower a VixiScript Cartesian coordinate literal to a hex-prism `(q, r, z)`.
///
/// A Cartesian coordinate literal is a 3-element array literal `[x, y, z]` —
/// the same `Vec<i64>` shape returned by the parser's (private) array-literal arm.
/// This wires that literal straight into the existing pp-math substrate
/// (`pp_math_v3::fixed_point::cartesian_to_hex_prism`) — the i64-upcast,
/// multiply-before-divide hex conversion. No math is reimplemented here.
///
/// `hex_size_mu` / `z_height_mu` are the grid cell dimensions in MilliUnits.
///
/// Returns `None` (rather than lowering) when:
/// - `literal` is not exactly 3 elements — it is not a coordinate, or
/// - `hex_size_mu == 0` — the substrate divides by `hex_size_mu` for `q`/`r`
///   and (unlike its `z_height_mu` path) does not guard that divisor, so a
///   zero here would panic. Guarding it at the lowering boundary keeps the
///   parser's "never panics on malformed input" contract intact.
pub fn lower_cartesian_to_hex_prism(
    literal: &[i64],
    hex_size_mu: i64,
    z_height_mu: i64,
) -> Option<(i64, i64, i64)> {
    if literal.len() != 3 || hex_size_mu == 0 {
        return None;
    }
    Some(pp_math_v3::fixed_point::cartesian_to_hex_prism(
        literal[0],
        literal[1],
        literal[2],
        hex_size_mu,
        z_height_mu,
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Atom block (the VixelAtom lowering-target join) ---------------------

    /// `parse_atom` — the `ast_arm` for the VixelAtom primitive. Proves an authored
    /// `atom {}` block parses to a typed `AtomDef` (coord/material_id/resonance/color):
    /// the frontend half of the join whose CST twin sings in the TKNO terminal.
    #[test]
    fn atom_block_parses_to_atom_def() {
        let ast = parse_vixel_source(
            "atom { coord: (12, 7), material_id: 3, resonance: 6000p, color: 0x40C0FF }",
            "atom.vixel",
        )
        .expect("atom block parses");
        assert_eq!(ast.atoms.len(), 1, "one atom authored");
        let a = &ast.atoms[0];
        assert_eq!(a.coord, (12, 7));
        assert_eq!(a.material_id, 3);
        assert_eq!(a.resonance, 6000);
        assert_eq!(a.color, 0x40C0FF);
    }

    /// An unknown atom property is a clean `ParseError`, never a panic (Signal Law).
    #[test]
    fn atom_rejects_unknown_property() {
        assert!(
            parse_vixel_source("atom { bogus: 1 }", "atom.vixel").is_err(),
            "unknown atom key must be a ParseError, not silently accepted"
        );
    }

    // -- Hex-prism lowering --------------------------------------------------

    /// A VixiScript Cartesian coordinate literal `[x, y, z]` lowers to the
    /// hex-prism `(q, r, z)` produced by `forge_core_v3::cartesian_to_hex_prism`.
    ///
    /// Parses the literal through the real `parse_array_literal` path, then
    /// lowers it — proving the end-to-end wiring, not just the math.
    ///
    /// Hand-computed for `[3000, 6000, 1500]`, hex_size=1000, z_height=1000:
    ///   q = (3000 * 6666) / (1000 * 10000) = 19_998_000 / 10_000_000 = 1
    ///   r = (-3000*3333 + 6000*5773) / 10_000_000
    ///     = (-9_999_000 + 34_638_000) / 10_000_000 = 24_639_000 / 1e7 = 2
    ///   z = 1500 / 1000 = 1
    #[test]
    fn cartesian_literal_lowers_to_hex_prism() {
        let mut parser = Parser::new("[3000, 6000, 1500]", "coord.vixel");
        let literal = parser.parse_array_literal().unwrap();
        assert_eq!(literal, vec![3000, 6000, 1500]);

        let lowered = lower_cartesian_to_hex_prism(&literal, 1000, 1000)
            .expect("3-element literal with non-zero hex_size lowers");
        assert_eq!(lowered, (1, 2, 1));
    }

    /// Negative Cartesian components lower correctly (parser supports unary
    /// minus in array elements; pp-math i64 path handles the sign).
    #[test]
    fn negative_cartesian_literal_lowers() {
        let mut parser = Parser::new("[-3000, 6000, 1500]", "coord.vixel");
        let literal = parser.parse_array_literal().unwrap();
        assert_eq!(literal, vec![-3000, 6000, 1500]);

        let lowered = lower_cartesian_to_hex_prism(&literal, 1000, 1000).unwrap();
        // q = (-3000*6666)/1e7 = -1 ; r = (3000*3333 + 6000*5773)/1e7 = 4
        assert_eq!(lowered, (-1, 4, 1));
    }

    /// Non-coordinate literals (wrong arity) do not lower.
    #[test]
    fn non_coordinate_literal_does_not_lower() {
        assert_eq!(lower_cartesian_to_hex_prism(&[1, 2], 1000, 1000), None);
        assert_eq!(lower_cartesian_to_hex_prism(&[1, 2, 3, 4], 1000, 1000), None);
    }

    /// A zero `hex_size_mu` would divide-by-zero in the substrate — the
    /// lowering guards it so the parser's no-panic contract holds.
    #[test]
    fn zero_hex_size_does_not_lower() {
        assert_eq!(lower_cartesian_to_hex_prism(&[3000, 6000, 1500], 0, 1000), None);
    }

    #[test]
    fn parse_empty_source() {
        let ast = parse_vixel_source("", "test.vixel").unwrap();
        assert!(ast.materials.is_empty());
        assert!(ast.spatials.is_empty());
        assert!(ast.automata.is_empty());
        assert!(ast.environment.is_empty());
    }

    #[test]
    fn parse_comment_only() {
        let src = "// this is a comment\n// another comment\n";
        let ast = parse_vixel_source(src, "test.vixel").unwrap();
        assert!(ast.materials.is_empty());
    }

    #[test]
    fn parse_material_block() {
        let src = r#"
material "oak" {
    mass: 4200,
    hardness: 3500,
    flammability: 7800,
    destruction: "splinter",
    albedo: 0x8B6914FF,
}
"#;
        let ast = parse_vixel_source(src, "test.vixel").unwrap();
        assert_eq!(ast.materials.len(), 1);
        let mat = &ast.materials[0];
        assert_eq!(mat.id, 0);
        assert_eq!(mat.name_str(), "oak");
        assert_eq!(mat.mass_pmy, 4200);
        assert_eq!(mat.hardness_pmy, 3500);
        assert_eq!(mat.flammability_pmy, 7800);
        assert_eq!(mat.destruction_mode, 1); // splinter
        assert_eq!(mat.albedo, 0x8B6914FF);
    }

    #[test]
    fn parse_multiple_materials_sequential_ids() {
        let src = r#"
material "oak" { mass: 4200, hardness: 3500, flammability: 7800, destruction: "splinter", albedo: 0x8B6914FF }
material "stone" { mass: 6000, hardness: 8000, flammability: 0, destruction: "shatter", albedo: 0x7A7268FF }
"#;
        let ast = parse_vixel_source(src, "test.vixel").unwrap();
        assert_eq!(ast.materials.len(), 2);
        assert_eq!(ast.materials[0].id, 0);
        assert_eq!(ast.materials[0].name_str(), "oak");
        assert_eq!(ast.materials[1].id, 1);
        assert_eq!(ast.materials[1].name_str(), "stone");
    }

    #[test]
    fn parse_spawn_grid() {
        let src = r#"spawn_grid("oak", 32, 32, 1);"#;
        let ast = parse_vixel_source(src, "test.vixel").unwrap();
        assert_eq!(ast.spatials.len(), 1);
        assert_eq!(ast.spatials[0].id, 0);
    }

    #[test]
    fn parse_spawn_forest_named_arg() {
        let src = r#"spawn_forest("oak", 500, radius: 64);"#;
        let ast = parse_vixel_source(src, "test.vixel").unwrap();
        assert_eq!(ast.spatials.len(), 1);
        assert_eq!(ast.spatials[0].stress_limit_pmy, 64);
    }

    #[test]
    fn parse_spawn_line_with_arrays() {
        let src = r#"spawn_line("stone", from: [0,0,0], to: [100,0,0]);"#;
        let ast = parse_vixel_source(src, "test.vixel").unwrap();
        assert_eq!(ast.spatials.len(), 1);
        let s = &ast.spatials[0];
        assert_eq!(s.socket_count, 2);
        assert_eq!(s.sockets[0], (0, 0, 0));
        assert_eq!(s.sockets[1], (100, 0, 0));
    }

    #[test]
    fn parse_rule_fire_spread() {
        let src = r#"
rule "fire_spread" {
    when: neighbor_has("fire") && self.flammability > 5000,
    then: set_material("fire"),
    tick_delay: 3,
}
"#;
        let ast = parse_vixel_source(src, "test.vixel").unwrap();
        assert_eq!(ast.automata.len(), 1);
        let rule = &ast.automata[0];
        assert_eq!(rule.id, 0);
        assert_eq!(rule.rule_type, AutomataType::Fire);
        assert!(rule.wgsl_source.contains("neighbor_has"));
        assert!(rule.wgsl_source.contains("set_material"));
    }

    #[test]
    fn parse_rule_gravity() {
        let src = r#"
rule "gravity" {
    when: below_is("air") && self.mass > 0,
    then: swap_with_below(),
    tick_delay: 1,
}
"#;
        let ast = parse_vixel_source(src, "test.vixel").unwrap();
        assert_eq!(ast.automata.len(), 1);
        assert_eq!(ast.automata[0].rule_type, AutomataType::Gravity);
    }

    #[test]
    fn parse_set_temperature() {
        let src = r#"set_temperature("fire", 9500);"#;
        let ast = parse_vixel_source(src, "test.vixel").unwrap();
        assert_eq!(ast.environment.len(), 1);
        let env = &ast.environment[0];
        assert_eq!(env.env_type, EnvironmentType::Temperature);
        assert_eq!(env.target_str(), "fire");
        assert_eq!(env.value_pmy, 9500);
    }

    #[test]
    fn parse_set_wind() {
        let src = r#"set_wind([1, 0, 0], strength: 3000);"#;
        let ast = parse_vixel_source(src, "test.vixel").unwrap();
        assert_eq!(ast.environment.len(), 1);
        let env = &ast.environment[0];
        assert_eq!(env.env_type, EnvironmentType::Wind);
        assert_eq!(env.vector, [1, 0, 0]);
        assert_eq!(env.value_pmy, 3000);
    }

    #[test]
    fn parse_set_gravity_env() {
        let src = r#"set_gravity([0, -1, 0], strength: 9800);"#;
        let ast = parse_vixel_source(src, "test.vixel").unwrap();
        assert_eq!(ast.environment.len(), 1);
        let env = &ast.environment[0];
        assert_eq!(env.env_type, EnvironmentType::Gravity);
        assert_eq!(env.vector, [0, -1, 0]);
        assert_eq!(env.value_pmy, 9800);
    }

    #[test]
    fn reject_float_literal() {
        let src = r#"
material "bad" {
    mass: 3.14,
}
"#;
        let result = parse_vixel_source(src, "test.vixel");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.message.contains("float"),
            "error should mention float: {}",
            err.message
        );
    }

    #[test]
    fn reject_float_in_spawn() {
        let src = r#"spawn_grid("oak", 3.14, 32, 1);"#;
        let result = parse_vixel_source(src, "test.vixel");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("float"));
    }

    #[test]
    fn error_includes_file_and_line() {
        let src = "material \"test\" {\n    mass: 3.14,\n}";
        let result = parse_vixel_source(src, "world.vixel");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.file, "world.vixel");
        assert_eq!(err.line, 2); // float is on line 2
    }

    #[test]
    fn error_on_unterminated_string() {
        let src = "material \"unterminated";
        let result = parse_vixel_source(src, "test.vixel");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("unterminated"));
    }

    #[test]
    fn error_on_unknown_keyword() {
        let src = "foobar 123;";
        let result = parse_vixel_source(src, "test.vixel");
        assert!(result.is_err());
    }

    #[test]
    fn parse_full_example() {
        let src = r#"
// Material definitions
material "oak" {
    mass: 4200,
    hardness: 3500,
    flammability: 7800,
    destruction: "splinter",
    albedo: 0x8B6914FF,
}

// Spatial placement
spawn_grid("oak", 32, 32, 1);
spawn_forest("oak", 500, radius: 64);
spawn_line("stone", from: [0,0,0], to: [100,0,0]);

// Physics rules (automata)
rule "fire_spread" {
    when: neighbor_has("fire") && self.flammability > 5000,
    then: set_material("fire"),
    tick_delay: 3,
}

// Environment
set_temperature("fire", 9500);
set_wind([1, 0, 0], strength: 3000);
set_gravity([0, -1, 0], strength: 9800);
"#;
        let ast = parse_vixel_source(src, "world.vixel").unwrap();
        assert_eq!(ast.materials.len(), 1);
        assert_eq!(ast.spatials.len(), 3);
        assert_eq!(ast.automata.len(), 1);
        assert_eq!(ast.environment.len(), 3);
    }

    #[test]
    fn automata_type_fire_detection() {
        assert_eq!(infer_automata_type("fire_spread when fire"), AutomataType::Fire);
        assert_eq!(infer_automata_type("burn flammability check"), AutomataType::Fire);
    }

    #[test]
    fn automata_type_fluid_detection() {
        assert_eq!(infer_automata_type("water_flow fluid sim"), AutomataType::Fluid);
    }

    #[test]
    fn automata_type_gravity_detection() {
        assert_eq!(infer_automata_type("gravity below check"), AutomataType::Gravity);
    }

    #[test]
    fn automata_type_custom_fallback() {
        assert_eq!(infer_automata_type("custom_rule something"), AutomataType::Custom);
    }

    #[test]
    fn destruction_modes() {
        let shatter = r#"material "a" { destruction: "shatter" }"#;
        let splinter = r#"material "b" { destruction: "splinter" }"#;
        let melt = r#"material "c" { destruction: "melt" }"#;

        let a = parse_vixel_source(shatter, "t").unwrap();
        let b = parse_vixel_source(splinter, "t").unwrap();
        let c = parse_vixel_source(melt, "t").unwrap();

        assert_eq!(a.materials[0].destruction_mode, 0);
        assert_eq!(b.materials[0].destruction_mode, 1);
        assert_eq!(c.materials[0].destruction_mode, 2);
    }

    #[test]
    fn unknown_destruction_mode_errors() {
        let src = r#"material "bad" { destruction: "explode" }"#;
        let result = parse_vixel_source(src, "t");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("destruction mode"));
    }

    #[test]
    fn parse_determinism() {
        // CP-1: Same input produces same output
        let src = r#"
material "oak" { mass: 4200, hardness: 3500, flammability: 7800, destruction: "splinter", albedo: 0x8B6914FF }
spawn_grid("oak", 32, 32, 1);
rule "fire_spread" { when: neighbor_has("fire") && self.flammability > 5000, then: set_material("fire"), tick_delay: 3 }
set_temperature("fire", 9500);
"#;
        let ast1 = parse_vixel_source(src, "test.vixel").unwrap();
        let ast2 = parse_vixel_source(src, "test.vixel").unwrap();
        assert_eq!(ast1, ast2);
    }

    #[test]
    fn parse_ui_block() {
        let src = r#"
ui "health_bar" {
    x: 10000,
    y: 5000,
    w: 200000,
    h: 12000,
    color: 0x50A060FF,
    material: 1,
    vibe: 0x01,
    radius: 4,
}
"#;
        let ast = parse_vixel_source(src, "test.vixel").unwrap();
        assert_eq!(ast.ui_defs.len(), 1);
        let ui = &ast.ui_defs[0];
        assert_eq!(ui.id, 0);
        assert_eq!(ui.name_str(), "health_bar");
        assert_eq!(ui.x, 10000);
        assert_eq!(ui.y, 5000);
        assert_eq!(ui.w, 200000);
        assert_eq!(ui.h, 12000);
        assert_eq!(ui.color, ColorValue::Literal(0x50A060FF));
        assert_eq!(ui.material_idx, 1);
        assert_eq!(ui.vibe_mask, 0x01);
        assert_eq!(ui.radius, 4);
    }

    #[test]
    fn parse_ui_mixed_with_materials() {
        let src = r#"
material "iron" { mass: 7800, hardness: 6000, flammability: 0, destruction: "shatter", albedo: 0x444444FF }
ui "panel" { x: 0, y: 0, w: 500000, h: 300000, color: 0x1A1A22FF }
"#;
        let ast = parse_vixel_source(src, "test.vixel").unwrap();
        assert_eq!(ast.materials.len(), 1);
        assert_eq!(ast.ui_defs.len(), 1);
        assert_eq!(ast.materials[0].name_str(), "iron");
        assert_eq!(ast.ui_defs[0].name_str(), "panel");
    }
}

