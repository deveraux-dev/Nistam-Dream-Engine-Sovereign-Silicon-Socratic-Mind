//! Traktor Kontrol S2 MK3 — USB HID driver.
//!
//! The S2 MK3 uses HID (not standard MIDI). VID=0x17CC, PID=0x1710.
//! Two input reports: 0x01 (buttons/jogwheels, 20 bytes), 0x02 (analog, 39 bytes).
//! One output report: 0x80 (LEDs, 62 bytes).

use std::sync::{Arc, Mutex};

const VID: u16 = 0x17CC;
const PID: u16 = 0x1710;

// --- Input events ---

#[derive(Debug, Clone)]
pub enum S2Event {
    /// Button pressed or released.
    Button { control: S2Button, deck: Deck, pressed: bool },
    /// Analog control changed (0.0–1.0).
    Analog { control: S2Analog, deck: Deck, value: f32 },
    /// Jog wheel delta (signed ticks).
    JogWheel { deck: Deck, delta: i32, touched: bool },
    /// Encoder rotation delta (Browse/Move/Loop knobs).
    Encoder { control: EncoderControl, delta: i8 },
    /// Deck focus changed (left and right columns assigned to different software decks).
    DeckFocusChanged { left: Deck, right: Deck },
    /// Beat auto-match triggered (crossfader + SHIFT).
    BeatAutoMatch { source: Deck, target: Deck },
}

/// Encoder knobs on S2 MK3 (report 0x01 bytes 9-11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncoderControl {
    Browse,
    Move,
    Loop,
}

/// Deck focus system — which software decks are controlled by left and right physical columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeckFocus {
    pub left: Deck,   // Which software deck the left physical column controls
    pub right: Deck,  // Which software deck the right physical column controls
}

impl Default for DeckFocus {
    fn default() -> Self {
        Self { left: Deck::A, right: Deck::B }
    }
}

/// Soft takeover — prevents parameter jumps when focus switches.
#[derive(Debug, Clone, Copy)]
pub struct SoftTakeover {
    pub target_value: f32,   // Software parameter value at time of focus switch
    pub captured: bool,      // Has the physical knob "caught up" yet?
}

impl Default for SoftTakeover {
    fn default() -> Self {
        Self { target_value: 0.0, captured: false }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Deck { A, B, C, D, Master }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum S2Button {
    Play, Cue, Sync, Shift, Keylock,
    Hotcue1, Hotcue2, Hotcue3, Hotcue4,
    Hotcue5, Hotcue6, Hotcue7, Hotcue8,
    Rev, Flx, LoadTrack, Grid, BrowseView,
    Pfl, BeatLoop, ReloopToggle, Quantize,
    Fx1, Fx2, Fx3, Fx4,
    Back, BrowseEncoderPush, Mic,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum S2Analog {
    Volume, Rate, Pregain,
    EqHigh, EqMid, EqLow,
    Crossfader, HeadphonesMix, HeadphonesVol,
    SuperKnob,
}

// --- LED output ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedColor {
    Off, Blue, Red, Green, Orange, White
}

impl LedColor {
    /// Convert to LED byte value for HID report.
    /// S2 MK3 hotcue pads are RGB — each pad has 3 consecutive bytes (R, G, B).
    /// Single-color LEDs (buttons, VU) just use brightness 0x00-0x7E.
    pub fn to_byte(self) -> u8 {
        match self {
            LedColor::Off => 0x7C,
            LedColor::Blue => 0x7E,
            LedColor::Red => 0x7E,
            LedColor::Green => 0x7E,
            LedColor::Orange => 0x7E,
            LedColor::White => 0x7E,
        }
    }
}

#[derive(Clone)]
pub struct S2Leds {
    pub data: [u8; 62],
}

impl Default for S2Leds {
    fn default() -> Self { Self { data: [0u8; 62] } }
}

impl S2Leds {
    pub fn set_play(&mut self, deck: Deck, on: bool) {
        // S2 MK3: 2 physical decks — C maps to A's LEDs, D maps to B's LEDs (deck focus layer)
        let off = match deck {
            Deck::A | Deck::C => 12,
            Deck::B | Deck::D => 51,
            _ => return,
        };
        self.data[off] = if on { 0x7E } else { 0x7C };
    }
    pub fn set_cue(&mut self, deck: Deck, on: bool) {
        let off = match deck {
            Deck::A | Deck::C => 11,
            Deck::B | Deck::D => 50,
            _ => return,
        };
        self.data[off] = if on { 0x7E } else { 0x7C };
    }
    pub fn set_sync(&mut self, deck: Deck, on: bool) {
        let off = match deck {
            Deck::A | Deck::C => 9,
            Deck::B | Deck::D => 48,
            _ => return,
        };
        self.data[off] = if on { 0x7E } else { 0x7C };
    }
    pub fn set_vu_meter(&mut self, deck: Deck, level: f32) {
        let base = match deck {
            Deck::A | Deck::C => 28,
            Deck::B | Deck::D => 34,
            _ => return,
        };
        let lvl = (level.clamp(0.0, 1.0) * 5.0) as usize;
        for i in 0..5 {
            self.data[base + i] = if i < lvl { 0x7E } else { 0x00 };
        }
    }
    /// Set FX button LED (slot 0-3).
    pub fn set_fx(&mut self, slot: usize, on: bool) {
        // FX buttons are at bytes 21-24 in the LED report
        if slot < 4 {
            self.data[21 + slot] = if on { 0x7E } else { 0x7C };
        }
    }
    /// Set hotcue LED for a specific deck and pad.
    pub fn set_hotcue(&mut self, deck: Deck, pad: usize, color: LedColor) {
        if pad > 7 { return; }
        let base = match deck {
            Deck::A | Deck::C => 13,
            Deck::B | Deck::D => 40,
            _ => return,
        };
        self.data[base + pad] = color.to_byte();
    }
    /// Set loop active indicator.
    pub fn set_loop_active(&mut self, deck: Deck, on: bool) {
        let off = match deck {
            Deck::A | Deck::C => 26,
            Deck::B | Deck::D => 32,
            _ => return,
        };
        self.data[off] = if on { 0x7E } else { 0x7C };
    }
    /// Set FX active indicator (slot 0-3).
    pub fn set_fx_active(&mut self, slot: usize, on: bool) {
        if slot < 4 {
            let off = 21 + slot;
            self.data[off] = if on { 0x7E } else { 0x7C };
        }
    }
    /// Set deck focus indicator LED.
    pub fn set_deck_focus_indicator(&mut self, deck: Deck, focused: bool) {
        // S2 MK3 has 2 physical deck focus indicators (left/right)
        // A/C share left indicator, B/D share right indicator
        let off = match deck {
            Deck::A | Deck::C => 60,
            Deck::B | Deck::D => 61,
            _ => return,
        };
        self.data[off] = if focused { 0x7E } else { 0x00 };
    }
    /// Set SHIFT indicator LED for a deck.
    pub fn set_shift(&mut self, deck: Deck, on: bool) {
        let off = match deck {
            Deck::A | Deck::C => 6,
            Deck::B | Deck::D => 45,
            _ => return,
        };
        self.data[off] = if on { 0x7E } else { 0x7C };
    }
    /// Set keylock indicator LED for a deck.
    pub fn set_keylock(&mut self, deck: Deck, on: bool) {
        let off = match deck {
            Deck::A | Deck::C => 8,
            Deck::B | Deck::D => 47,
            _ => return,
        };
        self.data[off] = if on { 0x7E } else { 0x7C };
    }
    /// Set PFL (pre-fader listen / headphone cue) indicator LED.
    pub fn set_pfl(&mut self, deck: Deck, on: bool) {
        let off = match deck {
            Deck::A | Deck::C => 25,
            Deck::B | Deck::D => 31,
            _ => return,
        };
        self.data[off] = if on { 0x7E } else { 0x7C };
    }
    /// Set flux/slip mode indicator LED.
    pub fn set_flux(&mut self, deck: Deck, on: bool) {
        let off = match deck {
            Deck::A | Deck::C => 7,
            Deck::B | Deck::D => 46,
            _ => return,
        };
        self.data[off] = if on { 0x7E } else { 0x7C };
    }
    /// Set reverse indicator LED.
    pub fn set_rev(&mut self, deck: Deck, on: bool) {
        let off = match deck {
            Deck::A | Deck::C => 10,
            Deck::B | Deck::D => 49,
            _ => return,
        };
        self.data[off] = if on { 0x7E } else { 0x7C };
    }
    /// Set mic indicator LED (global, byte 57).
    pub fn set_mic(&mut self, on: bool) {
        self.data[57] = if on { 0x7E } else { 0x7C };
    }
    /// Browse LED (byte 60) — lit when browse panel is open.
    pub fn set_browse(&mut self, on: bool) {
        self.data[60] = if on { 0x7E } else { 0x7C };
    }
    pub fn all_on(&mut self) {
        for b in &mut self.data[1..] { *b = 0x7E; }
    }
}

// --- HID connection ---

pub type S2EventQueue = Arc<Mutex<Vec<S2Event>>>;

pub fn new_event_queue() -> S2EventQueue {
    Arc::new(Mutex::new(Vec::new()))
}

/// Try to open the S2 MK3 HID device.
/// The S2 MK3 exposes multiple USB interfaces — we need the one with
/// usage_page 0xFF01 (vendor-defined) which carries the control data.
pub fn open_device(api: &hidapi::HidApi) -> Result<hidapi::HidDevice, String> {
    // First, try to find the vendor-defined usage page (0xFF01)
    for info in api.device_list() {
        if info.vendor_id() == VID && info.product_id() == PID {
            eprintln!("[S2] Found interface {} usage_page=0x{:04x} usage=0x{:04x}",
                info.interface_number(), info.usage_page(), info.usage());
            if info.usage_page() == 0xFF01 {
                eprintln!("[S2] Opening vendor-defined interface {}", info.interface_number());
                return info.open_device(api)
                    .map_err(|e| format!("S2 HID open (iface {}): {}", info.interface_number(), e));
            }
        }
    }
    // Fallback: try each interface
    for info in api.device_list() {
        if info.vendor_id() == VID && info.product_id() == PID {
            eprintln!("[S2] Fallback: trying interface {}", info.interface_number());
            if let Ok(dev) = info.open_device(api) {
                return Ok(dev);
            }
        }
    }
    Err(format!("S2 HID: no device found (VID={:04x} PID={:04x})", VID, PID))
}

/// List available HID devices (for debugging).
pub fn list_hid_devices() -> Vec<String> {
    let Ok(api) = hidapi::HidApi::new() else { return vec![] };
    api.device_list()
        .map(|d| format!("{:04x}:{:04x} iface={} usage_page=0x{:04x} usage=0x{:04x} {} {}",
            d.vendor_id(), d.product_id(),
            d.interface_number(),
            d.usage_page(), d.usage(),
            d.manufacturer_string().unwrap_or("?"),
            d.product_string().unwrap_or("?")))
        .collect()
}

// --- Report parsing ---

/// Previous state for delta detection.
#[derive(Default)]
pub struct HidPrevState {
    buttons_1: [u8; 9],  // bytes 1-8 of report 0x01
    analog_2: [u16; 19], // 16-bit values from report 0x02
    encoders: [u8; 3],   // bytes 9-11 of report 0x01 (Browse/Move/Loop)
    jog_a: u32,
    jog_b: u32,
    jog_touch_a: bool,
    jog_touch_b: bool,
    first: bool,
    _debug_count: u32,
}

fn read_u16_le(buf: &[u8], offset: usize) -> u16 {
    if offset + 1 < buf.len() {
        u16::from_le_bytes([buf[offset], buf[offset + 1]])
    } else {
        0
    }
}

fn read_u32_le(buf: &[u8], offset: usize) -> u32 {
    if offset + 3 < buf.len() {
        u32::from_le_bytes([buf[offset], buf[offset + 1], buf[offset + 2], buf[offset + 3]])
    } else {
        0
    }
}

fn check_button(prev: u8, cur: u8, mask: u8, control: S2Button, deck: Deck, events: &mut Vec<S2Event>) {
    let was = prev & mask != 0;
    let now = cur & mask != 0;
    if was != now {
        events.push(S2Event::Button { control, deck, pressed: now });
    }
}

fn parse_report_01(buf: &[u8], prev: &mut HidPrevState, events: &mut Vec<S2Event>) {
    if buf.len() < 20 { return; }

    // Deck A buttons — byte 1
    check_button(prev.buttons_1[0], buf[1], 0x01, S2Button::Rev, Deck::A, events);
    check_button(prev.buttons_1[0], buf[1], 0x02, S2Button::Flx, Deck::A, events);
    check_button(prev.buttons_1[0], buf[1], 0x04, S2Button::LoadTrack, Deck::A, events);  // "Preparation"
    check_button(prev.buttons_1[0], buf[1], 0x08, S2Button::BrowseView, Deck::A, events);
    check_button(prev.buttons_1[0], buf[1], 0x10, S2Button::Grid, Deck::A, events);
    check_button(prev.buttons_1[0], buf[1], 0x20, S2Button::Shift, Deck::A, events);

    // Deck A buttons — byte 2
    check_button(prev.buttons_1[1], buf[2], 0x01, S2Button::Sync, Deck::A, events);
    check_button(prev.buttons_1[1], buf[2], 0x02, S2Button::Keylock, Deck::A, events);
    check_button(prev.buttons_1[1], buf[2], 0x04, S2Button::Cue, Deck::A, events);
    check_button(prev.buttons_1[1], buf[2], 0x08, S2Button::Play, Deck::A, events);
    check_button(prev.buttons_1[1], buf[2], 0x10, S2Button::Hotcue1, Deck::A, events);
    check_button(prev.buttons_1[1], buf[2], 0x20, S2Button::Hotcue2, Deck::A, events);
    check_button(prev.buttons_1[1], buf[2], 0x40, S2Button::Hotcue3, Deck::A, events);
    check_button(prev.buttons_1[1], buf[2], 0x80, S2Button::Hotcue4, Deck::A, events);

    // Deck A hotcues 5-8 + FX — byte 3
    check_button(prev.buttons_1[2], buf[3], 0x01, S2Button::Hotcue5, Deck::A, events);
    check_button(prev.buttons_1[2], buf[3], 0x02, S2Button::Hotcue6, Deck::A, events);
    check_button(prev.buttons_1[2], buf[3], 0x04, S2Button::Hotcue7, Deck::A, events);
    check_button(prev.buttons_1[2], buf[3], 0x08, S2Button::Hotcue8, Deck::A, events);
    check_button(prev.buttons_1[2], buf[3], 0x10, S2Button::Fx1, Deck::Master, events);
    check_button(prev.buttons_1[2], buf[3], 0x20, S2Button::Fx2, Deck::Master, events);
    check_button(prev.buttons_1[2], buf[3], 0x40, S2Button::Fx3, Deck::Master, events);
    check_button(prev.buttons_1[2], buf[3], 0x80, S2Button::Fx4, Deck::Master, events);

    // PFL — byte 4
    check_button(prev.buttons_1[3], buf[4], 0x01, S2Button::Pfl, Deck::A, events);
    check_button(prev.buttons_1[3], buf[4], 0x02, S2Button::Pfl, Deck::B, events);

    // Deck B buttons — byte 4
    check_button(prev.buttons_1[3], buf[4], 0x04, S2Button::Rev, Deck::B, events);
    check_button(prev.buttons_1[3], buf[4], 0x08, S2Button::Flx, Deck::B, events);
    check_button(prev.buttons_1[3], buf[4], 0x10, S2Button::LoadTrack, Deck::B, events);  // "Preparation"
    check_button(prev.buttons_1[3], buf[4], 0x20, S2Button::BrowseView, Deck::B, events);
    check_button(prev.buttons_1[3], buf[4], 0x40, S2Button::Grid, Deck::B, events);
    check_button(prev.buttons_1[3], buf[4], 0x80, S2Button::Shift, Deck::B, events);

    // Deck B buttons — byte 5
    check_button(prev.buttons_1[4], buf[5], 0x04, S2Button::Sync, Deck::B, events);
    check_button(prev.buttons_1[4], buf[5], 0x08, S2Button::Keylock, Deck::B, events);
    check_button(prev.buttons_1[4], buf[5], 0x10, S2Button::Cue, Deck::B, events);
    check_button(prev.buttons_1[4], buf[5], 0x20, S2Button::Play, Deck::B, events);
    check_button(prev.buttons_1[4], buf[5], 0x40, S2Button::Hotcue1, Deck::B, events);
    check_button(prev.buttons_1[4], buf[5], 0x80, S2Button::Hotcue2, Deck::B, events);

    // Deck B hotcues 3-8 — byte 6
    check_button(prev.buttons_1[5], buf[6], 0x01, S2Button::Hotcue3, Deck::B, events);
    check_button(prev.buttons_1[5], buf[6], 0x02, S2Button::Hotcue4, Deck::B, events);
    check_button(prev.buttons_1[5], buf[6], 0x04, S2Button::Hotcue5, Deck::B, events);
    check_button(prev.buttons_1[5], buf[6], 0x08, S2Button::Hotcue6, Deck::B, events);
    check_button(prev.buttons_1[5], buf[6], 0x10, S2Button::Hotcue7, Deck::B, events);
    check_button(prev.buttons_1[5], buf[6], 0x20, S2Button::Hotcue8, Deck::B, events);
    check_button(prev.buttons_1[5], buf[6], 0x40, S2Button::Quantize, Deck::Master, events);

    // Byte 7 — Back, Browse encoder push, Mic
    check_button(prev.buttons_1[6], buf[7], 0x02, S2Button::Back, Deck::Master, events);
    check_button(prev.buttons_1[6], buf[7], 0x04, S2Button::BrowseEncoderPush, Deck::Master, events);
    check_button(prev.buttons_1[6], buf[7], 0x08, S2Button::Mic, Deck::Master, events);

    // Encoders — bytes 9-11 (Browse, Move, Loop)
    // Each byte is a rotation delta encoded as unsigned; convert to signed.
    // 0 = no movement, 1+ = clockwise, 0xFF(-1) = counter-clockwise.
    const ENCODER_MAP: [(usize, EncoderControl); 3] = [
        (9,  EncoderControl::Browse),
        (10, EncoderControl::Move),
        (11, EncoderControl::Loop),
    ];
    for (idx, &(byte_off, ref control)) in ENCODER_MAP.iter().enumerate() {
        let raw = buf[byte_off];
        if raw != prev.encoders[idx] {
            let delta = raw as i8; // wrapping: 0xFF → -1, 0x01 → 1
            if delta != 0 {
                events.push(S2Event::Encoder { control: *control, delta });
            }
            prev.encoders[idx] = raw;
        }
    }

    // Jog touch — byte 8
    let touch_a = buf[8] & 0x40 != 0;
    let touch_b = buf[8] & 0x80 != 0;

    // Jog wheels — bytes 12-15 (A), 16-19 (B), 32-bit
    let jog_a = read_u32_le(buf, 12);
    let jog_b = read_u32_le(buf, 16);

    let calc_delta = |curr_u32: u32, prev_u32: u32| -> i32 {
        let tickval = (curr_u32 & 0xFF) as i32;
        let prev_tick = (prev_u32 & 0xFF) as i32;
        if prev_tick >= 200 && tickval <= 100 {
            tickval + 256 - prev_tick
        } else if prev_tick <= 100 && tickval >= 200 {
            tickval - prev_tick - 256
        } else {
            tickval - prev_tick
        }
    };

    if prev.first {
        if jog_a != prev.jog_a {
            let delta = calc_delta(jog_a, prev.jog_a);
            events.push(S2Event::JogWheel { deck: Deck::A, delta, touched: touch_a });
        }
        if jog_b != prev.jog_b {
            let delta = calc_delta(jog_b, prev.jog_b);
            events.push(S2Event::JogWheel { deck: Deck::B, delta, touched: touch_b });
        }
    }

    if touch_a != prev.jog_touch_a {
        events.push(S2Event::JogWheel { deck: Deck::A, delta: 0, touched: touch_a });
    }
    if touch_b != prev.jog_touch_b {
        events.push(S2Event::JogWheel { deck: Deck::B, delta: 0, touched: touch_b });
    }

    // Save state
    prev.buttons_1[0..8].copy_from_slice(&buf[1..9]);
    prev.jog_a = jog_a;
    prev.jog_b = jog_b;
    prev.jog_touch_a = touch_a;
    prev.jog_touch_b = touch_b;
    prev.first = true;
}

fn parse_report_02(buf: &[u8], prev: &mut HidPrevState, events: &mut Vec<S2Event>) {
    if buf.len() < 39 { return; }

    // Analog controls: 16-bit LE, range 0–4095 → 0.0–1.0
    let controls: &[(usize, S2Analog, Deck, usize)] = &[
        (1,  S2Analog::Rate,         Deck::A,  0),
        (3,  S2Analog::Volume,       Deck::A,  1),
        (5,  S2Analog::Crossfader,   Deck::Master, 2),
        (7,  S2Analog::Volume,       Deck::B,  3),
        (9,  S2Analog::Rate,         Deck::B,  4),
        (11, S2Analog::Pregain,      Deck::A,  5),
        (13, S2Analog::EqHigh,       Deck::A,  6),
        (15, S2Analog::EqMid,        Deck::A,  7),
        (17, S2Analog::EqLow,        Deck::A,  8),
        (19, S2Analog::SuperKnob,    Deck::A,  9),
        (25, S2Analog::HeadphonesMix,Deck::Master, 10),
        (27, S2Analog::HeadphonesVol,Deck::Master, 11),
        (29, S2Analog::Pregain,      Deck::B,  12),
        (31, S2Analog::EqHigh,       Deck::B,  13),
        (33, S2Analog::EqMid,        Deck::B,  14),
        (35, S2Analog::EqLow,        Deck::B,  15),
        (37, S2Analog::SuperKnob,    Deck::B,  16),
    ];

    for &(offset, ref control, deck, idx) in controls {
        let raw = read_u16_le(buf, offset);
        if raw != prev.analog_2[idx] {
            let value = raw as f32 / 4095.0;
            events.push(S2Event::Analog { control: *control, deck, value });
            prev.analog_2[idx] = raw;
        }
    }
}

/// Parse a single HID report buffer into events.
pub fn parse_report(buf: &[u8], prev: &mut HidPrevState) -> Vec<S2Event> {
    let mut events = Vec::new();
    match buf[0] {
        0x01 => parse_report_01(buf, prev, &mut events),
        0x02 => parse_report_02(buf, prev, &mut events),
        _ => {}
    }
    events
}

/// Send LED state to the controller.
pub fn send_leds(device: &hidapi::HidDevice, leds: &S2Leds) -> Result<(), String> {
    let mut report = [0u8; 63]; // report ID + 62 bytes
    report[0] = 0x80;
    report[1..].copy_from_slice(&leds.data);
    device.write(&report).map_err(|e| format!("HID LED write: {}", e))?;
    Ok(())
}

/// Drain all pending events from the queue.
pub fn drain_events(queue: &S2EventQueue) -> Vec<S2Event> {
    queue.lock().map(|mut q| q.drain(..).collect()).unwrap_or_default()
}

use crate::controller::{ControllerEvent, ControllerDriver};

/// S2 HID controller driver — adapts S2Events to ControllerEvent.
pub struct S2HidDriver {
    event_queue: S2EventQueue,
    device: Option<hidapi::HidDevice>,
    prev_state: HidPrevState,
    pub leds: S2Leds,
    pub focus: DeckFocus,
    /// Soft takeover state for each parameter to prevent jumps on focus switch.
    soft_takeover: std::collections::HashMap<String, SoftTakeover>,
    _shift_pressed: bool,
}

impl S2HidDriver {
    /// Connect to the S2 hardware.
    pub fn connect() -> Result<Self, String> {
        let api = hidapi::HidApi::new()
            .map_err(|e| format!("HID init: {}", e))?;
        let device = open_device(&api)?;
        device.set_blocking_mode(false)
            .map_err(|e| format!("S2 non-blocking: {}", e))?;
        Ok(Self {
            event_queue: new_event_queue(),
            device: Some(device),
            prev_state: HidPrevState::default(),
            leds: S2Leds::default(),
            focus: DeckFocus::default(),
            soft_takeover: std::collections::HashMap::new(),
            _shift_pressed: false,
        })
    }

    /// Create without hardware (for testing).
    pub fn new_from_queue(queue: S2EventQueue) -> Self {
        Self {
            event_queue: queue,
            device: None,
            prev_state: HidPrevState::default(),
            leds: S2Leds::default(),
            focus: DeckFocus::default(),
            soft_takeover: std::collections::HashMap::new(),
            _shift_pressed: false,
        }
    }

    /// Handle a focus toggle event.
    /// SHIFT + Left PFL/CUE button → toggle left physical deck between A↔C
    /// SHIFT + Right PFL/CUE button → toggle right physical deck between B↔D
    pub fn toggle_focus(&mut self, side: Deck) -> Option<S2Event> {
        match side {
            Deck::A => {
                let new_left = if self.focus.left == Deck::A { Deck::C } else { Deck::A };
                self.focus.left = new_left;
                Some(S2Event::DeckFocusChanged { 
                    left: self.focus.left,
                    right: self.focus.right,
                })
            }
            Deck::B => {
                let new_right = if self.focus.right == Deck::B { Deck::D } else { Deck::B };
                self.focus.right = new_right;
                Some(S2Event::DeckFocusChanged {
                    left: self.focus.left,
                    right: self.focus.right,
                })
            }
            _ => None,
        }
    }

    /// Record a soft takeover checkpoint for a parameter when focus switches.
    pub fn record_soft_takeover(&mut self, param_id: String, value: f32) {
        self.soft_takeover.insert(param_id, SoftTakeover {
            target_value: value,
            captured: false,
        });
    }

    /// Check if a parameter has captured the physical control (soft takeover complete).
    pub fn check_soft_takeover_capture(&mut self, param_id: &str, physical_value: f32) -> bool {
        if let Some(st) = self.soft_takeover.get_mut(param_id) {
            if !st.captured {
                // Consider "captured" if the physical knob is close enough to the software value
                st.captured = (physical_value - st.target_value).abs() < 0.05;
            }
            st.captured
        } else {
            true // If no soft takeover is active, accept the value
        }
    }

    fn deck_str(deck: &Deck) -> &'static str {
        match deck {
            Deck::A => "a",
            Deck::B => "b",
            Deck::C => "c",
            Deck::D => "d",
            Deck::Master => "master",
        }
    }

    fn analog_str(control: &S2Analog) -> &'static str {
        match control {
            S2Analog::Volume => "volume",
            S2Analog::Rate => "rate",
            S2Analog::Pregain => "pregain",
            S2Analog::EqHigh => "eq_high",
            S2Analog::EqMid => "eq_mid",
            S2Analog::EqLow => "eq_low",
            S2Analog::Crossfader => "crossfader",
            S2Analog::HeadphonesMix => "headphones_mix",
            S2Analog::HeadphonesVol => "headphones_vol",
            S2Analog::SuperKnob => "super_knob",
        }
    }

    fn button_str(control: &S2Button) -> &'static str {
        match control {
            S2Button::Play => "play",
            S2Button::Cue => "cue",
            S2Button::Sync => "sync",
            S2Button::Shift => "shift",
            S2Button::Keylock => "keylock",
            S2Button::Hotcue1 => "hotcue1",
            S2Button::Hotcue2 => "hotcue2",
            S2Button::Hotcue3 => "hotcue3",
            S2Button::Hotcue4 => "hotcue4",
            S2Button::Hotcue5 => "hotcue5",
            S2Button::Hotcue6 => "hotcue6",
            S2Button::Hotcue7 => "hotcue7",
            S2Button::Hotcue8 => "hotcue8",
            S2Button::Rev => "rev",
            S2Button::Flx => "flx",
            S2Button::LoadTrack => "load_track",
            S2Button::Grid => "grid",
            S2Button::BrowseView => "browse_view",
            S2Button::Pfl => "pfl",
            S2Button::BeatLoop => "beat_loop",
            S2Button::ReloopToggle => "reloop_toggle",
            S2Button::Quantize => "quantize",
            S2Button::Fx1 => "fx1",
            S2Button::Fx2 => "fx2",
            S2Button::Fx3 => "fx3",
            S2Button::Fx4 => "fx4",
            S2Button::Back => "back",
            S2Button::BrowseEncoderPush => "browse_encoder_push",
            S2Button::Mic => "mic",
        }
    }

    pub fn translate(event: &S2Event) -> Vec<ControllerEvent> {
        match event {
            S2Event::Analog { control, deck, value } => {
                vec![ControllerEvent::Analog {
                    source_id: format!("s2:{}:{}", Self::deck_str(deck), Self::analog_str(control)),
                    value: *value,
                }]
            }
            S2Event::Button { control, deck, pressed } => {
                vec![ControllerEvent::Button {
                    source_id: format!("s2:{}:{}", Self::deck_str(deck), Self::button_str(control)),
                    pressed: *pressed,
                }]
            }
            S2Event::JogWheel { deck, delta, .. } => {
                vec![ControllerEvent::Relative {
                    source_id: format!("s2:{}:jog", Self::deck_str(deck)),
                    delta: *delta as f32,
                }]
            }
            S2Event::Encoder { control, delta } => {
                vec![ControllerEvent::Relative {
                    source_id: format!("s2:encoder:{:?}", control).to_lowercase(),
                    delta: *delta as f32,
                }]
            }
            // System events don't translate to ControllerEvent
            S2Event::DeckFocusChanged { .. } | S2Event::BeatAutoMatch { .. } => {
                vec![]
            }
        }
    }

    /// Send current LED state to the hardware.
    pub fn set_leds(&self) -> Result<(), String> {
        if let Some(ref dev) = self.device {
            send_leds(dev, &self.leds)
        } else {
            Ok(())
        }
    }
}

impl ControllerDriver for S2HidDriver {
    fn name(&self) -> &str { "Kontrol S2" }

    fn poll(&mut self) -> Vec<ControllerEvent> {
        let mut disconnected = false;
        if let Some(ref device) = self.device {
            let mut buf = [0u8; 64];
            loop {
                match device.read(&mut buf) {
                    Ok(len) if len == 0 => break, // No more events
                    Ok(len) => {
                        let events = parse_report(&buf[..len], &mut self.prev_state);
                        if let Ok(mut q) = self.event_queue.lock() {
                            q.extend(events);
                        }
                    }
                    Err(e) => {
                        eprintln!("[S2 MK3] CONTROLLER LOST: Read error ({}). Sticky fallback engaged.", e);
                        disconnected = true;
                        break;
                    }
                }
            }
        }

        if disconnected {
            self.device = None;
        }

        drain_events(&self.event_queue)
            .iter()
            .flat_map(Self::translate)
            .collect()
    }

    fn connected(&self) -> bool {
        self.device.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_play_button() {
        let mut prev = HidPrevState::default();
        prev.first = true;
        let mut events = Vec::new();

        // Report 0x01 with Play A pressed (byte 2, bit 0x08)
        let mut buf = [0u8; 20];
        buf[0] = 0x01;
        buf[2] = 0x08; // Play A
        parse_report_01(&buf, &mut prev, &mut events);

        assert_eq!(events.len(), 1);
        match &events[0] {
            S2Event::Button { control: S2Button::Play, deck: Deck::A, pressed: true } => {}
            other => panic!("Expected Play A press, got {:?}", other),
        }
    }

    #[test]
    fn parse_analog_crossfader() {
        let mut prev = HidPrevState::default();
        let mut events = Vec::new();

        // Report 0x02 with crossfader at midpoint (2048 = 0x0800)
        let mut buf = [0u8; 39];
        buf[0] = 0x02;
        buf[5] = 0x00; // low byte
        buf[6] = 0x08; // high byte → 0x0800 = 2048
        parse_report_02(&buf, &mut prev, &mut events);

        let cf_event = events.iter().find(|e| matches!(e,
            S2Event::Analog { control: S2Analog::Crossfader, .. }
        ));
        assert!(cf_event.is_some());
        if let S2Event::Analog { value, .. } = cf_event.unwrap() {
            assert!((value - 0.5).abs() < 0.01);
        }
    }

    #[test]
    fn s2_analog_to_controller_event() {
        use crate::controller::ControllerEvent;
        let events = S2HidDriver::translate(&S2Event::Analog {
            control: S2Analog::EqHigh,
            deck: Deck::A,
            value: 0.75,
        });
        assert_eq!(events.len(), 1);
        match &events[0] {
            ControllerEvent::Analog { source_id, value } => {
                assert_eq!(source_id, "s2:a:eq_high");
                assert!((value - 0.75).abs() < 0.001);
            }
            other => panic!("Expected Analog, got {:?}", other),
        }
    }

    #[test]
    fn s2_button_to_controller_event() {
        use crate::controller::ControllerEvent;
        let events = S2HidDriver::translate(&S2Event::Button {
            control: S2Button::Play,
            deck: Deck::B,
            pressed: true,
        });
        assert_eq!(events.len(), 1);
        match &events[0] {
            ControllerEvent::Button { source_id, pressed } => {
                assert_eq!(source_id, "s2:b:play");
                assert!(*pressed);
            }
            other => panic!("Expected Button, got {:?}", other),
        }
    }

    #[test]
    fn s2_jog_to_relative_event() {
        use crate::controller::ControllerEvent;
        let events = S2HidDriver::translate(&S2Event::JogWheel {
            deck: Deck::A,
            delta: -15,
            touched: true,
        });
        assert_eq!(events.len(), 1);
        match &events[0] {
            ControllerEvent::Relative { source_id, delta } => {
                assert_eq!(source_id, "s2:a:jog");
                assert_eq!(*delta, -15.0);
            }
            other => panic!("Expected Relative, got {:?}", other),
        }
    }

    #[test]
    fn s2_master_deck_source_id() {
        use crate::controller::ControllerEvent;
        let events = S2HidDriver::translate(&S2Event::Analog {
            control: S2Analog::Crossfader,
            deck: Deck::Master,
            value: 0.5,
        });
        match &events[0] {
            ControllerEvent::Analog { source_id, .. } => {
                assert_eq!(source_id, "s2:master:crossfader");
            }
            other => panic!("Expected Analog, got {:?}", other),
        }
    }

    #[test]
    fn s2_jog_touch_only() {
        use crate::controller::ControllerEvent;
        let events = S2HidDriver::translate(&S2Event::JogWheel {
            deck: Deck::B,
            delta: 0,
            touched: true,
        });
        assert_eq!(events.len(), 1);
        match &events[0] {
            ControllerEvent::Relative { delta, .. } => assert_eq!(*delta, 0.0),
            other => panic!("Expected Relative, got {:?}", other),
        }
    }

    // SPEC 01: New tests for 4-deck support
    #[test]
    fn test_deck_focus_default() {
        let driver = S2HidDriver::new_from_queue(new_event_queue());
        assert_eq!(driver.focus.left, Deck::A);
        assert_eq!(driver.focus.right, Deck::B);
    }

    #[test]
    fn test_deck_focus_shift_swap() {
        let mut driver = S2HidDriver::new_from_queue(new_event_queue());
        
        // Toggle left: A → C
        if let Some(S2Event::DeckFocusChanged { left, right }) = driver.toggle_focus(Deck::A) {
            assert_eq!(left, Deck::C);
            assert_eq!(right, Deck::B);
        } else {
            panic!("Expected DeckFocusChanged event");
        }
        
        // Toggle back: C → A
        if let Some(S2Event::DeckFocusChanged { left, right }) = driver.toggle_focus(Deck::A) {
            assert_eq!(left, Deck::A);
            assert_eq!(right, Deck::B);
        } else {
            panic!("Expected DeckFocusChanged event");
        }

        // Toggle right: B → D
        if let Some(S2Event::DeckFocusChanged { left, right }) = driver.toggle_focus(Deck::B) {
            assert_eq!(left, Deck::A);
            assert_eq!(right, Deck::D);
        } else {
            panic!("Expected DeckFocusChanged event");
        }
    }

    #[test]
    fn test_all_buttons_mapped() {
        // Verify every S2Button variant will produce a valid deck_str output
        let buttons = [
            S2Button::Play, S2Button::Cue, S2Button::Sync, S2Button::Shift, S2Button::Keylock,
            S2Button::Hotcue1, S2Button::Hotcue2, S2Button::Hotcue3, S2Button::Hotcue4,
            S2Button::Hotcue5, S2Button::Hotcue6, S2Button::Hotcue7, S2Button::Hotcue8,
            S2Button::Rev, S2Button::Flx, S2Button::LoadTrack, S2Button::Grid, S2Button::BrowseView,
            S2Button::Pfl, S2Button::BeatLoop, S2Button::ReloopToggle, S2Button::Quantize,
            S2Button::Fx1, S2Button::Fx2, S2Button::Fx3, S2Button::Fx4,
            S2Button::Back, S2Button::BrowseEncoderPush, S2Button::Mic,
        ];
        
        for btn in &buttons {
            let s = S2HidDriver::button_str(btn);
            assert!(!s.is_empty(), "Button {:?} has no string representation", btn);
        }
    }

    #[test]
    fn test_all_analogs_mapped() {
        // Verify every S2Analog variant will produce a valid analog_str output
        let analogs = [
            S2Analog::Volume, S2Analog::Rate, S2Analog::Pregain,
            S2Analog::EqHigh, S2Analog::EqMid, S2Analog::EqLow,
            S2Analog::Crossfader, S2Analog::HeadphonesMix, S2Analog::HeadphonesVol,
            S2Analog::SuperKnob,
        ];
        
        for anlg in &analogs {
            let s = S2HidDriver::analog_str(anlg);
            assert!(!s.is_empty(), "Analog {:?} has no string representation", anlg);
        }
    }

    #[test]
    fn test_led_play_all_decks() {
        let mut leds = S2Leds::default();
        
        // Verify we can set play for all decks without panicking
        leds.set_play(Deck::A, true);
        leds.set_play(Deck::B, true);
        // C and D don't have direct LED mappings on S2 MK3 (only A/B present)
        
        assert_eq!(leds.data[12], 0x7E); // Deck A play should be on
        assert_eq!(leds.data[51], 0x7E); // Deck B play should be on
    }

    #[test]
    fn test_led_hotcue_colors() {
        let mut leds = S2Leds::default();
        
        // Test all LedColor variants produce valid bytes (0x7C=off, 0x7E=on)
        assert_eq!(LedColor::Off.to_byte(), 0x7C);
        assert_eq!(LedColor::Blue.to_byte(), 0x7E);
        assert_eq!(LedColor::Red.to_byte(), 0x7E);
        
        // Test setting hotcues on multiple decks
        leds.set_hotcue(Deck::A, 0, LedColor::Blue);
        leds.set_hotcue(Deck::A, 1, LedColor::Off);
        leds.set_hotcue(Deck::B, 0, LedColor::Red);
        
        assert_eq!(leds.data[13], 0x7E); // Hotcue 0 on Deck A = on
        assert_eq!(leds.data[14], 0x7C); // Hotcue 1 on Deck A = off
        assert_eq!(leds.data[40], 0x7E); // Hotcue 0 on Deck B = on
    }

    #[test]
    fn test_controller_driver_poll() {
        use crate::controller::ControllerDriver;
        let queue = new_event_queue();
        
        // Inject an S2Event into the queue
        if let Ok(mut q) = queue.lock() {
            q.push(S2Event::Button {
                control: S2Button::Play,
                deck: Deck::A,
                pressed: true,
            });
        }
        
        let mut driver = S2HidDriver::new_from_queue(queue);
        let events = driver.poll();
        
        assert_eq!(events.len(), 1);
        match &events[0] {
            ControllerEvent::Button { source_id, pressed } => {
                assert_eq!(source_id, "s2:a:play");
                assert!(*pressed);
            }
            _ => panic!("Expected Button event from poll()"),
        }
    }

    #[test]
    fn test_beat_auto_match_event() {
        let evt = S2Event::BeatAutoMatch { source: Deck::A, target: Deck::C };
        
        // Verify event is created correctly
        match evt {
            S2Event::BeatAutoMatch { source, target } => {
                assert_eq!(source, Deck::A);
                assert_eq!(target, Deck::C);
            }
            _ => panic!("Expected BeatAutoMatch event"),
        }
        
        // Verify it translates to empty (system event, not a parameter change)
        let translated = S2HidDriver::translate(&evt);
        assert_eq!(translated.len(), 0);
    }

    #[test]
    fn test_soft_takeover_basic() {
        let mut driver = S2HidDriver::new_from_queue(new_event_queue());
        
        // Record a soft takeover at 0.5
        driver.record_soft_takeover("eq_high".to_string(), 0.5);
        
        // Physical knob not captured initially
        assert!(!driver.check_soft_takeover_capture("eq_high", 0.3));
        
        // Knob moves closer to target
        assert!(!driver.check_soft_takeover_capture("eq_high", 0.45));
        
        // Knob crosses or reaches target
        assert!(driver.check_soft_takeover_capture("eq_high", 0.5));
    }

    #[test]
    fn test_all_deck_strings() {
        // Verify all Deck variants have string representations
        for deck in [Deck::A, Deck::B, Deck::C, Deck::D, Deck::Master] {
            let s = S2HidDriver::deck_str(&deck);
            assert!(!s.is_empty(), "Deck {:?} has no string representation", deck);
        }
    }
}
