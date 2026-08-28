//! Hidden account names — the 13 classified ledger accounts.
//! Revealed through disclosure surfaces, never in normal UI.

use crate::lore::RevealStage;

/// One classified hidden ledger account's definition.
#[derive(Clone, Copy, Debug)]
pub struct HiddenAccountDef {
    /// Stable machine-readable account identifier.
    pub account_id: &'static str,
    /// The account's true, classified name.
    pub hidden_name: &'static str,
    /// The public-facing hint shown before disclosure.
    pub public_hint: &'static str,
    /// The account's thematic content.
    pub theme: &'static str,
    /// The disclosure stage at which this account is revealed.
    pub reveal_stage: RevealStage,
}

/// The 13 classified hidden ledger accounts.
pub const HIDDEN_ACCOUNTS: &[HiddenAccountDef] = &[
    HiddenAccountDef { account_id: "RedDebt", hidden_name: "The Blood Owed Before Birth", public_hint: "red account", theme: "violence, inherited guilt, unpaid taking", reveal_stage: RevealStage::Proof },
    HiddenAccountDef { account_id: "StoneRoot", hidden_name: "The Root That Would Not Kneel", public_hint: "stone account", theme: "burden, endurance, land-binding, oath weight", reveal_stage: RevealStage::Proof },
    HiddenAccountDef { account_id: "DoubleWitness", hidden_name: "The Two Who Saw Otherwise", public_hint: "split account", theme: "contradiction, testimony, unreliable proof", reveal_stage: RevealStage::Proof },
    HiddenAccountDef { account_id: "GraveWater", hidden_name: "The River Beneath the Grave", public_hint: "wet account", theme: "mourning, burial, memory seepage, ancestral debt", reveal_stage: RevealStage::Proof },
    HiddenAccountDef { account_id: "CrownlessRoar", hidden_name: "The King Without a Throat", public_hint: "broken crown", theme: "authority, failed rule, silenced sovereignty", reveal_stage: RevealStage::Proof },
    HiddenAccountDef { account_id: "CleanIndex", hidden_name: "The Hand That Files the Dead", public_hint: "clean ledger", theme: "bureaucracy, erasure, lawful atrocity", reveal_stage: RevealStage::Proof },
    HiddenAccountDef { account_id: "EqualKnife", hidden_name: "The Blade That Makes All Debts Equal", public_hint: "level knife", theme: "justice, revenge, false balance", reveal_stage: RevealStage::Proof },
    HiddenAccountDef { account_id: "VenomWedding", hidden_name: "The Bridegroom in the Poison Cup", public_hint: "green vow", theme: "binding, betrayal, marriage-as-contract", reveal_stage: RevealStage::Proof },
    HiddenAccountDef { account_id: "FarWound", hidden_name: "The Injury That Arrives Before the Arrow", public_hint: "distant scar", theme: "prophecy, causality, remote harm", reveal_stage: RevealStage::Proof },
    HiddenAccountDef { account_id: "LastToll", hidden_name: "The Bell Paid Once at the End", public_hint: "final toll", theme: "thresholds, passage, price of return", reveal_stage: RevealStage::Proof },
    HiddenAccountDef { account_id: "HollowStar", hidden_name: "The Star With No Witness Inside", public_hint: "empty star", theme: "void, fame, hollow divinity", reveal_stage: RevealStage::Proof },
    HiddenAccountDef { account_id: "MercyDrowned", hidden_name: "The Hand That Held Mercy Underwater", public_hint: "drowned mercy", theme: "failed compassion, sacrifice, corrupted healing", reveal_stage: RevealStage::Proof },
    HiddenAccountDef { account_id: "OutsideWheel", hidden_name: "The Thirteenth Spoke That Turns Nothing", public_hint: "outside wheel", theme: "classification failure, anti-zodiac, void exception", reveal_stage: RevealStage::Absence },
];
