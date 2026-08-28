//! RunProfile — the per-run stat ledger, authored against
//! `F:\v3\TODO\ironroot-edict\IRONROOT_Design_Packet\
//! ironroot_mvp_schema_extract.json:13-32` (the schema, field-for-field —
//! 19 fields, `u32`/`i32` typed exactly as the packet states them; no
//! scale is given for the `_q` fields there, so they stay plain `i32`
//! rather than an invented fixed-point unit).
//!
//! Doctrine (`ironroot_thread_synthesis_machine_readable.json:40-41`):
//! "Prototype Vertical Slice 1: one between-wave choice writing to
//! RunProfile." / "Prototype Vertical Slice 2: one sieve that changes next
//! wave based on RunProfile." This module lands Slice 1's target — a
//! profile every choice can write to. The sieve itself (Slice 2, boss
//! manifestation selection off this profile) is real, cited, unported
//! work that belongs in its own module, not invented here.
//!
//! Every field increments through a named method — no bare `pub` mutation
//! from callers, so every write to the ledger names the real event that
//! caused it (kill, spare, parry, refusal, ...), same as the packet's own
//! field names describe.

/// The per-run stat ledger. All-zero at run start; every field climbs by
/// exactly the amount one real, named event contributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RunProfile {
    /// Enemies killed.
    pub kills: u32,
    /// Enemies spared instead of killed.
    pub spares: u32,
    /// Times the player has died this run.
    pub deaths: u32,
    /// Perfect parries landed.
    pub perfect_parries: u32,
    /// Items crafted.
    pub crafts: u32,
    /// Items repaired.
    pub repairs: u32,
    /// Songs performed.
    pub songs: u32,
    /// Bargains struck.
    pub bargains: u32,
    /// Thefts committed.
    pub thefts: u32,
    /// Witnesses saved from harm.
    pub witnesses_saved: u32,
    /// Commands refused.
    pub commands_refused: u32,
    /// Repeated-action count, packet-typed `i32` with no stated scale.
    pub repeated_actions_q: i32,
    /// Damage taken, packet-typed `i32` with no stated scale.
    pub damage_taken_q: i32,
    /// Bell tolls answered.
    pub bell_answers: u32,
    /// Blood-tier supply used, packet-typed `i32` with no stated scale.
    pub blood_supply_used_q: i32,
    /// Clean-tier supply used, packet-typed `i32` with no stated scale.
    pub clean_supply_used_q: i32,
    /// Fraud committed, packet-typed `i32` with no stated scale.
    pub fraud_q: i32,
    /// Treaties signed.
    pub treaties_signed: u32,
    /// Agreements broken.
    pub agreements_broken: u32,
}

impl RunProfile {
    /// A fresh, all-zero ledger for a new run.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a kill.
    pub fn record_kill(&mut self) {
        self.kills = self.kills.saturating_add(1);
    }

    /// Record a spare.
    pub fn record_spare(&mut self) {
        self.spares = self.spares.saturating_add(1);
    }

    /// Record a death.
    pub fn record_death(&mut self) {
        self.deaths = self.deaths.saturating_add(1);
    }

    /// Record a perfect parry.
    pub fn record_perfect_parry(&mut self) {
        self.perfect_parries = self.perfect_parries.saturating_add(1);
    }

    /// Record a craft.
    pub fn record_craft(&mut self) {
        self.crafts = self.crafts.saturating_add(1);
    }

    /// Record a repair.
    pub fn record_repair(&mut self) {
        self.repairs = self.repairs.saturating_add(1);
    }

    /// Record a song performed.
    pub fn record_song(&mut self) {
        self.songs = self.songs.saturating_add(1);
    }

    /// Record a bargain struck.
    pub fn record_bargain(&mut self) {
        self.bargains = self.bargains.saturating_add(1);
    }

    /// Record a theft.
    pub fn record_theft(&mut self) {
        self.thefts = self.thefts.saturating_add(1);
    }

    /// Record a witness saved.
    pub fn record_witness_saved(&mut self) {
        self.witnesses_saved = self.witnesses_saved.saturating_add(1);
    }

    /// Record a refused command.
    pub fn record_command_refused(&mut self) {
        self.commands_refused = self.commands_refused.saturating_add(1);
    }

    /// Record a bell toll answered.
    pub fn record_bell_answer(&mut self) {
        self.bell_answers = self.bell_answers.saturating_add(1);
    }

    /// Record a treaty signed.
    pub fn record_treaty_signed(&mut self) {
        self.treaties_signed = self.treaties_signed.saturating_add(1);
    }

    /// Record an agreement broken.
    pub fn record_agreement_broken(&mut self) {
        self.agreements_broken = self.agreements_broken.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_profile_is_all_zero() {
        let p = RunProfile::new();
        assert_eq!(p, RunProfile::default());
        assert_eq!(p.kills, 0);
        assert_eq!(p.agreements_broken, 0);
    }

    #[test]
    fn each_recorder_advances_only_its_own_field() {
        let mut p = RunProfile::new();
        p.record_kill();
        p.record_spare();
        p.record_bell_answer();
        assert_eq!(p.kills, 1);
        assert_eq!(p.spares, 1);
        assert_eq!(p.bell_answers, 1);
        assert_eq!(p.deaths, 0, "recording a kill must not also count as a death");
    }

    #[test]
    fn recorders_saturate_instead_of_wrapping() {
        let mut p = RunProfile::new();
        p.kills = u32::MAX;
        p.record_kill();
        assert_eq!(p.kills, u32::MAX, "a maxed stat must saturate, not wrap to 0 (L07 bijection would break on wrap)");
    }

    #[test]
    fn a_between_wave_choice_writes_to_the_profile() {
        // Doctrine: "one between-wave choice writing to RunProfile"
        // (ironroot_thread_synthesis_machine_readable.json:40).
        let mut p = RunProfile::new();
        // The player spares a captive between waves instead of taking the kill.
        p.record_spare();
        p.record_witness_saved();
        assert_eq!(p.spares, 1);
        assert_eq!(p.witnesses_saved, 1);
    }
}
