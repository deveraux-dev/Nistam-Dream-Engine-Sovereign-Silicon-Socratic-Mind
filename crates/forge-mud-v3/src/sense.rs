//! look-through-sense — mechanism ported 2026-08-26 from v2 sf-wasm
//! (mud.rs:1236 sense_here / :1385 do_look): look consumes a sensed view of
//! RESIDENT live organs; absent state reads "not installed", never "quiet".

/// The room as sensed off live state — the tell data look's prose was built
/// from, so a consumer renders without re-parsing prose (donor RoomView shape).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoomView {
    /// Sensory tell lines, sense order: square talk, shadow, sieve weather.
    pub tells: Vec<String>,
}

/// Live readings the organs surrendered this tick. Permyriad where suffixed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SenseReadings {
    /// `SocialRoom::level_q` — how disturbed the square's talk is.
    pub square_level_q: i64,
    /// `SocialRoom::is_disturbed` — the talk has not resumed.
    pub square_disturbed: bool,
    /// `ShadowMemory::pressure_q` — how hard the haunt leans on this node.
    pub haunt_pressure_q: i32,
    /// `ShadowMemory::aggression_level` — how far past watching it is.
    pub haunt_aggression: u8,
    /// `WeatherSieve` droughts held, in sieve ticks.
    pub drought_ticks: u32,
    /// `WeatherSieve` blizzard hold, in sieve ticks.
    pub blizzard_ticks: u32,
    /// `WeatherSieve` chinook pressure building on the high air.
    pub chinook_buildup: i32,
}

/// Tells from readings. Pure and total: zero readings earn zero tells.
pub fn sense(r: &SenseReadings) -> RoomView {
    let mut tells = Vec::new();
    if r.square_disturbed {
        tells.push(String::from("the talk has stopped; the square is watching you."));
    } else if r.square_level_q > 0 {
        tells.push(String::from("the talk dips a half-note as you pass."));
    }
    if r.haunt_aggression > 0 {
        tells.push(String::from("your shadow drags a half-step behind, unwilling."));
    } else if r.haunt_pressure_q > 0 {
        tells.push(String::from("the hair on your neck answers something unseen."));
    }
    if r.blizzard_ticks > 0 {
        tells.push(String::from("the cold here has teeth, and patience."));
    } else if r.drought_ticks > 0 {
        tells.push(String::from("the ground gives up its dust too easily."));
    }
    if r.chinook_buildup > 0 {
        tells.push(String::from("a warm wind leans on the high air; something is about to break."));
    }
    RoomView { tells }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_readings_earn_zero_tells() {
        assert!(sense(&SenseReadings::default()).tells.is_empty());
    }

    #[test]
    fn a_disturbed_square_outranks_a_dipped_one() {
        let dipped = sense(&SenseReadings { square_level_q: 100, ..Default::default() });
        let stopped = sense(&SenseReadings {
            square_level_q: 5000,
            square_disturbed: true,
            ..Default::default()
        });
        assert_ne!(dipped.tells[0], stopped.tells[0]);
        assert_eq!(dipped.tells.len(), 1);
        assert_eq!(stopped.tells.len(), 1);
    }

    #[test]
    fn every_organ_contributes_at_most_one_tell() {
        let all = sense(&SenseReadings {
            square_level_q: 5000,
            square_disturbed: true,
            haunt_pressure_q: 900,
            haunt_aggression: 2,
            drought_ticks: 4,
            blizzard_ticks: 9,
            chinook_buildup: 3,
        });
        assert_eq!(all.tells.len(), 4, "square + shadow + cold + chinook: {:?}", all.tells);
    }
}
