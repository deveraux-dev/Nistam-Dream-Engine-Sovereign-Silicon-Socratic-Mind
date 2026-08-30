//! Calendar — an in-world calendar: integer day maps to a season and an era. The
//! narrative clock behind the weather pages.

use crate::weather::Era;
use serde::{Deserialize, Serialize};

/// An integer calendar over the four eras.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Calendar {
    /// Current day count in the calendar.
    pub day: u64,
    /// Number of days comprising each era cycle.
    pub days_per_era: u64,
}

impl Calendar {
    /// Creates a new calendar with the given era duration, minimum 4 days.
    pub fn new(days_per_era: u64) -> Self {
        Self { day: 0, days_per_era: days_per_era.max(4) }
    }

    /// Advances the calendar by the given number of days and returns the new day count.
    pub fn advance(&mut self, days: u64) -> u64 {
        self.day += days;
        self.day
    }

    /// The current era (cycles ancient -> golden -> decay -> void).
    pub fn era(&self) -> Era {
        match (self.day / self.days_per_era) % 4 {
            0 => Era::Ancient,
            1 => Era::Golden,
            2 => Era::Decay,
            _ => Era::Void,
        }
    }

    /// Returns the day count within the current era (0 to days_per_era - 1).
    pub fn day_of_era(&self) -> u64 {
        self.day % self.days_per_era
    }

    /// One of four seasons within the era.
    pub fn season(&self) -> &'static str {
        let quarter = self.day_of_era() / (self.days_per_era / 4);
        match quarter.min(3) {
            0 => "spring",
            1 => "summer",
            2 => "autumn",
            _ => "winter",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn era_cycles_over_days() {
        let mut c = Calendar::new(100);
        assert_eq!(c.era(), Era::Ancient);
        c.advance(100);
        assert_eq!(c.era(), Era::Golden);
        c.advance(300);
        assert_eq!(c.era(), Era::Ancient); // wrapped
    }

    #[test]
    fn seasons_quarter_the_era() {
        let mut c = Calendar::new(100);
        assert_eq!(c.season(), "spring");
        c.advance(50);
        assert_eq!(c.season(), "autumn");
        assert_eq!(c.day_of_era(), 50);
    }
}
