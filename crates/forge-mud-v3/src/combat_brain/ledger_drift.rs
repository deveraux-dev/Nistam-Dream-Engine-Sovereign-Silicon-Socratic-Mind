//! Ledger Drift — hidden zodiac accumulator driven by player actions.
//!
//! Ported by translation from forge-cart-brain::ledger_drift. The player never
//! sees "Aries +3". They see consequences. Internally, every meaningful action
//! deposits into one of 13 accounts.

/// The 13 ledger accounts (machine names, never surfaced as player-facing words).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LedgerAccount {
    /// Red Debt account.
    RedDebt,
    /// Stone Root account.
    StoneRoot,
    /// Double Witness account.
    DoubleWitness,
    /// Grave Water account.
    GraveWater,
    /// Crownless Roar account.
    CrownlessRoar,
    /// Clean Index account.
    CleanIndex,
    /// Equal Knife account.
    EqualKnife,
    /// Venom Wedding account.
    VenomWedding,
    /// Far Wound account.
    FarWound,
    /// Last Toll account.
    LastToll,
    /// Hollow Star account.
    HollowStar,
    /// Mercy Drowned account.
    MercyDrowned,
    /// Outside Wheel account.
    OutsideWheel,
}

/// The ledger state — 13 integer counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LedgerDrift {
    /// Red Debt counter.
    pub red_debt: i32,
    /// Stone Root counter.
    pub stone_root: i32,
    /// Double Witness counter.
    pub double_witness: i32,
    /// Grave Water counter.
    pub grave_water: i32,
    /// Crownless Roar counter.
    pub crownless_roar: i32,
    /// Clean Index counter.
    pub clean_index: i32,
    /// Equal Knife counter.
    pub equal_knife: i32,
    /// Venom Wedding counter.
    pub venom_wedding: i32,
    /// Far Wound counter.
    pub far_wound: i32,
    /// Last Toll counter.
    pub last_toll: i32,
    /// Hollow Star counter.
    pub hollow_star: i32,
    /// Mercy Drowned counter.
    pub mercy_drowned: i32,
    /// Outside Wheel counter.
    pub outside_wheel: i32,
}

impl LedgerDrift {
    /// Apply a signed delta to a specific account.
    pub fn apply(&mut self, account: LedgerAccount, amount: i32) {
        match account {
            LedgerAccount::RedDebt => self.red_debt += amount,
            LedgerAccount::StoneRoot => self.stone_root += amount,
            LedgerAccount::DoubleWitness => self.double_witness += amount,
            LedgerAccount::GraveWater => self.grave_water += amount,
            LedgerAccount::CrownlessRoar => self.crownless_roar += amount,
            LedgerAccount::CleanIndex => self.clean_index += amount,
            LedgerAccount::EqualKnife => self.equal_knife += amount,
            LedgerAccount::VenomWedding => self.venom_wedding += amount,
            LedgerAccount::FarWound => self.far_wound += amount,
            LedgerAccount::LastToll => self.last_toll += amount,
            LedgerAccount::HollowStar => self.hollow_star += amount,
            LedgerAccount::MercyDrowned => self.mercy_drowned += amount,
            LedgerAccount::OutsideWheel => self.outside_wheel += amount,
        }
    }

    /// Returns the dominant account (highest absolute value).
    pub fn dominant(&self) -> LedgerAccount {
        let vals = [
            (self.red_debt, LedgerAccount::RedDebt),
            (self.stone_root, LedgerAccount::StoneRoot),
            (self.double_witness, LedgerAccount::DoubleWitness),
            (self.grave_water, LedgerAccount::GraveWater),
            (self.crownless_roar, LedgerAccount::CrownlessRoar),
            (self.clean_index, LedgerAccount::CleanIndex),
            (self.equal_knife, LedgerAccount::EqualKnife),
            (self.venom_wedding, LedgerAccount::VenomWedding),
            (self.far_wound, LedgerAccount::FarWound),
            (self.last_toll, LedgerAccount::LastToll),
            (self.hollow_star, LedgerAccount::HollowStar),
            (self.mercy_drowned, LedgerAccount::MercyDrowned),
            (self.outside_wheel, LedgerAccount::OutsideWheel),
        ];
        vals.iter().max_by_key(|(v, _)| v.abs()).map(|(_, a)| *a).unwrap_or(LedgerAccount::OutsideWheel)
    }

    /// Total sum of all accounts.
    pub fn total(&self) -> i32 {
        self.red_debt
            + self.stone_root
            + self.double_witness
            + self.grave_water
            + self.crownless_roar
            + self.clean_index
            + self.equal_knife
            + self.venom_wedding
            + self.far_wound
            + self.last_toll
            + self.hollow_star
            + self.mercy_drowned
            + self.outside_wheel
    }
}

/// Drift event categories — the actions that trigger ledger changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriftEvent {
    /// Used execution event.
    UsedExecution,
    /// Refused execution event.
    RefusedExecution,
    /// Restored name event.
    RestoredName,
    /// Spent name for power event.
    SpentNameForPower,
    /// Burned ledger site event.
    BurnedLedgerSite,
    /// Sealed zone event.
    SealedZone,
    /// Split record event.
    SplitRecord,
    /// Built witness chain event.
    BuiltWitnessChain,
    /// Forgave debtor event.
    ForgaveDebtor,
    /// Accepted faction office event.
    AcceptedFactionOffice,
    /// Refused faction office event.
    RefusedFactionOffice,
    /// Opened far route event.
    OpenedFarRoute,
    /// Broke correspondence event.
    BrokeCorrespondence,
    /// Preserved morrow collar event.
    PreservedMorrowCollar,
    /// Crafted artifact event.
    CraftedArtifact,
    /// Used death route event.
    UsedDeathRoute,
    /// Stolen ledger entry event.
    StolenLedgerEntry,
    /// Balanced harm event.
    BalancedHarm,
}

/// Apply a drift event to the ledger. Returns which account was affected.
pub fn apply_drift(drift: &mut LedgerDrift, event: DriftEvent) -> LedgerAccount {
    let (account, amount) = match event {
        DriftEvent::UsedExecution => (LedgerAccount::RedDebt, 3),
        DriftEvent::RefusedExecution => (LedgerAccount::OutsideWheel, 5),
        DriftEvent::RestoredName => (LedgerAccount::GraveWater, 4),
        DriftEvent::SpentNameForPower => (LedgerAccount::RedDebt, 2),
        DriftEvent::BurnedLedgerSite => (LedgerAccount::RedDebt, 4),
        DriftEvent::SealedZone => (LedgerAccount::StoneRoot, 4),
        DriftEvent::SplitRecord => (LedgerAccount::DoubleWitness, 4),
        DriftEvent::BuiltWitnessChain => (LedgerAccount::CrownlessRoar, 3),
        DriftEvent::ForgaveDebtor => (LedgerAccount::MercyDrowned, 4),
        DriftEvent::AcceptedFactionOffice => (LedgerAccount::LastToll, 3),
        DriftEvent::RefusedFactionOffice => (LedgerAccount::OutsideWheel, 4),
        DriftEvent::OpenedFarRoute => (LedgerAccount::FarWound, 4),
        DriftEvent::BrokeCorrespondence => (LedgerAccount::HollowStar, 4),
        DriftEvent::PreservedMorrowCollar => (LedgerAccount::OutsideWheel, 8),
        DriftEvent::CraftedArtifact => (LedgerAccount::StoneRoot, 2),
        DriftEvent::UsedDeathRoute => (LedgerAccount::GraveWater, 2),
        DriftEvent::StolenLedgerEntry => (LedgerAccount::DoubleWitness, 3),
        DriftEvent::BalancedHarm => (LedgerAccount::EqualKnife, 4),
    };
    drift.apply(account, amount);
    account
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drift_starts_empty() {
        let d = LedgerDrift::default();
        assert_eq!(d.total(), 0);
    }

    #[test]
    fn execution_feeds_red_debt() {
        let mut d = LedgerDrift::default();
        apply_drift(&mut d, DriftEvent::UsedExecution);
        assert_eq!(d.red_debt, 3);
        assert_eq!(d.dominant(), LedgerAccount::RedDebt);
    }

    #[test]
    fn refusal_feeds_outside_wheel() {
        let mut d = LedgerDrift::default();
        apply_drift(&mut d, DriftEvent::RefusedExecution);
        apply_drift(&mut d, DriftEvent::RefusedFactionOffice);
        apply_drift(&mut d, DriftEvent::PreservedMorrowCollar);
        assert_eq!(d.dominant(), LedgerAccount::OutsideWheel);
    }

    #[test]
    fn apply_delta_to_account() {
        let mut d = LedgerDrift::default();
        d.apply(LedgerAccount::StoneRoot, 10);
        assert_eq!(d.stone_root, 10);
        d.apply(LedgerAccount::StoneRoot, -5);
        assert_eq!(d.stone_root, 5);
    }

    #[test]
    fn dominant_tracks_highest_absolute_value() {
        let mut d = LedgerDrift::default();
        d.red_debt = -100;
        d.stone_root = 50;
        assert_eq!(d.dominant(), LedgerAccount::RedDebt);
    }

    #[test]
    fn total_sums_all_accounts() {
        let mut d = LedgerDrift::default();
        d.red_debt = 10;
        d.stone_root = -5;
        d.double_witness = 20;
        let sum = 10 - 5 + 20;
        assert_eq!(d.total(), sum);
    }
}
