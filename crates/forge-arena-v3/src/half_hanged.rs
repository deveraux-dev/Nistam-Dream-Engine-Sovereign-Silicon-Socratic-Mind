//! Half-Hanged liminal death state.
//!
//! When HP <= 0: player enters HalfHanged for 600 ticks (10 sec).
//! Only action: "Vicious Tongue" AoE INT debuff.
//! On timer expire: transition to Dead.
//! All coordinates in millimeters (i64). No f32.

use super::config::*;
use super::state::PlayerPhase;

/// Ghost movement speed in mm/tick (reduced from normal).
const GHOST_SPEED_MM: i64 = 2000;

/// Vicious Tongue cooldown in ticks.
const VICIOUS_TONGUE_COOLDOWN: u8 = 60;

/// Process movement and Vicious Tongue for a Half-Hanged player.
/// Returns Some(sound_id) if Vicious Tongue was triggered this tick.
pub fn execute_half_hanged_logic(
    phase: &mut PlayerPhase,
    x: &mut i64,
    y: &mut i64,
    input: u8,
) -> Option<u32> {
    let mut sound = None;

    if let PlayerPhase::HalfHanged { ref mut trauma_cooldown, .. } = phase {
        // Ghost movement (reduced speed, no gravity)
        if input & INPUT_RIGHT != 0 { *x += GHOST_SPEED_MM; }
        if input & INPUT_LEFT != 0 { *x -= GHOST_SPEED_MM; }
        if input & INPUT_UP != 0 { *y -= GHOST_SPEED_MM; }
        if input & INPUT_DOWN != 0 { *y += GHOST_SPEED_MM; }

        if *trauma_cooldown > 0 {
            *trauma_cooldown -= 1;
        } else if input & INPUT_ATTACK != 0 {
            *trauma_cooldown = VICIOUS_TONGUE_COOLDOWN;
            sound = Some(50); // Vicious Tongue SFX
        }
    }

    sound
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ghost_moves_right() {
        let mut phase = PlayerPhase::HalfHanged { ticks_remaining: 600, trauma_cooldown: 0 };
        let mut x: i64 = 0;
        let mut y: i64 = 0;
        execute_half_hanged_logic(&mut phase, &mut x, &mut y, INPUT_RIGHT);
        assert_eq!(x, GHOST_SPEED_MM);
    }

    #[test]
    fn vicious_tongue_fires_and_cooldown() {
        let mut phase = PlayerPhase::HalfHanged { ticks_remaining: 600, trauma_cooldown: 0 };
        let mut x: i64 = 0;
        let mut y: i64 = 0;

        let sound = execute_half_hanged_logic(&mut phase, &mut x, &mut y, INPUT_ATTACK);
        assert_eq!(sound, Some(50));

        // Should be on cooldown now
        let sound2 = execute_half_hanged_logic(&mut phase, &mut x, &mut y, INPUT_ATTACK);
        assert_eq!(sound2, None);
    }
}
