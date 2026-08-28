//! The thirteen hidden accounts, drained from forge-insights
//! `rpg/account.rs:10-77` (`Account` enum, `.name()`/`.reading()`).
//! `Account::open_on(seed, beat)` there is `ALL[mix(seed ^
//! beat.rotate_left(17)) % 13]`; `game::Game::status` ports that exact
//! shape — `Operator::xp` (monotonic "terminal bytes earned",
//! operator.rs:88-89) stands in for `beat`, and this crate's own
//! `operator::seed_hash` (the one house mixer, L05) stands in for
//! forge-insights' private `mix` — same seed-XOR-rotated-beat structure,
//! this crate's own mixing primitive.

/// The thirteen accounts: (name, reading), in the source's own `ALL` order.
pub const ACCOUNTS: &[(&str, &str)] = &[
    ("Red Debt", "Something was carried and not written down."),
    ("Stone Root", "What you set will still be standing after you."),
    ("Double Witness", "Two saw it. Neither will say so first."),
    ("Grave-Water", "The water under the orchard has been rising."),
    ("Crownless Roar", "Loud, and answering to nobody."),
    ("Clean Index", "Nothing owed. It reads strange when it happens."),
    ("Equal Knife", "The same edge for whoever holds it."),
    ("Venom Wedding", "A bargain made. Both parties poisoned."),
    ("Far Wound", "It was done a long way from here."),
    ("Last Toll", "The bell has one ring left in it."),
    ("Hollow Star", "A light with nothing behind it."),
    ("Mercy Drowned", "Mercy was offered. It went under."),
    ("Outside the Wheel", "Off the ledger. No one is coming."),
];

#[cfg(test)]
mod tests {
    #[test]
    fn accounts_are_exactly_thirteen_and_word_only() {
        assert_eq!(super::ACCOUNTS.len(), 13);
        for (name, reading) in super::ACCOUNTS {
            assert!(!name.is_empty() && name.is_ascii());
            assert!(!reading.is_empty() && reading.is_ascii());
            assert!(!name.chars().any(|c| c.is_ascii_digit()));
            assert!(!reading.chars().any(|c| c.is_ascii_digit()));
        }
    }
}
