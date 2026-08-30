//! Input bitmask and tick-rate constants.

pub const INPUT_RIGHT: u8 = 0x01;
pub const INPUT_LEFT: u8 = 0x02;
pub const INPUT_UP: u8 = 0x04;
pub const INPUT_DOWN: u8 = 0x08;
pub const INPUT_JUMP: u8 = 0x10;
pub const INPUT_ATTACK: u8 = 0x20;
pub const INPUT_DASH: u8 = 0x40;
pub const INPUT_SKILL: u8 = 0x80;

pub const TICK_RATE_HZ: u32 = 60;

pub const fn ticks_from_secs(seconds: u32) -> u32 {
    seconds * TICK_RATE_HZ
}

pub const fn ticks_from_ms(ms: u32) -> u32 {
    ms * TICK_RATE_HZ / 1000
}
