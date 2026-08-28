//! Ledger Drift — hidden zodiac accumulator driven by player actions.
//!
//! The player never sees "Aries +3". They see consequences.
//! Internally, every meaningful action deposits into one of 13 accounts.

/// One of 13 hidden ledger accounts that accumulate player karma.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LedgerAccount {
    /// Debt incurred through bloodshed.
    RedDebt,
    /// Karma from steadfast, rooted action.
    StoneRoot,
    /// Karma from acting under conflicting witnesses.
    DoubleWitness,
    /// Karma tied to burial/death sites.
    GraveWater,
    /// Karma from unclaimed, roaring defiance.
    CrownlessRoar,
    /// Karma from clean, verified transactions.
    CleanIndex,
    /// Karma from fair, balanced exchange.
    EqualKnife,
    /// Karma from corrupted bonds.
    VenomWedding,
    /// Karma from wounds sustained far from home.
    FarWound,
    /// Karma from a final reckoning.
    LastToll,
    /// Karma tied to isolation.
    HollowStar,
    /// Karma from mercy shown to the drowning.
    MercyDrowned,
    /// Karma from acting outside the sanctioned cycle.
    OutsideWheel,
}

/// The accumulated drift across all 13 accounts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LedgerDrift {
    /// Red Debt account.
    pub red_debt: i32,
    /// Stone Root account.
    pub stone_root: i32,
    /// Double Witness account.
    pub double_witness: i32,
    /// Grave Water account.
    pub grave_water: i32,
    /// Crownless Roar account.
    pub crownless_roar: i32,
    /// Clean Index account.
    pub clean_index: i32,
    /// Equal Knife account.
    pub equal_knife: i32,
    /// Venom Wedding account.
    pub venom_wedding: i32,
    /// Far Wound account.
    pub far_wound: i32,
    /// Last Toll account.
    pub last_toll: i32,
    /// Hollow Star account.
    pub hollow_star: i32,
    /// Mercy Drowned account.
    pub mercy_drowned: i32,
    /// Outside Wheel account.
    pub outside_wheel: i32,
}

impl LedgerDrift {
    /// Apply a signed amount to a specific account.
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

    /// Sum of all accounts.
    pub fn total(&self) -> i32 {
        self.red_debt + self.stone_root + self.double_witness + self.grave_water
            + self.crownless_roar + self.clean_index + self.equal_knife + self.venom_wedding
            + self.far_wound + self.last_toll + self.hollow_star + self.mercy_drowned
            + self.outside_wheel
    }
}

// ── Drift Events ─────────────────────────────────────────────────────────────

/// A significant player action that deposits into the ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriftEvent {
    /// Player carried out an execution.
    UsedExecution,
    /// Player declined to execute.
    RefusedExecution,
    /// Player restored a struck name.
    RestoredName,
    /// Player spent their name for power.
    SpentNameForPower,
    /// Player burned a ledger site.
    BurnedLedgerSite,
    /// Player sealed off a zone.
    SealedZone,
    /// Player split a record in two.
    SplitRecord,
    /// Player built a chain of witnesses.
    BuiltWitnessChain,
    /// Player forgave a debtor.
    ForgaveDebtor,
    /// Player accepted a faction office.
    AcceptedFactionOffice,
    /// Player refused a faction office.
    RefusedFactionOffice,
    /// Player opened a far, distant route.
    OpenedFarRoute,
    /// Player broke off correspondence.
    BrokeCorrespondence,
    /// Player preserved a morrow collar.
    PreservedMorrowCollar,
    /// Player crafted an artifact.
    CraftedArtifact,
    /// Player used the death route.
    UsedDeathRoute,
    /// Player stole a ledger entry.
    StolenLedgerEntry,
    /// Player balanced a harm done.
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
}
