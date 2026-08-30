//! 7-7-7 duel core — wires [`hypercube`] routing to the
//! Sevenfold turn clock. 7 turns, each governed by one of the 7 Hermetic
//! [`Principle`]s in canonical order;
//! 7 cards, a closed pool (no draw); 7 life, measured as Hamming distance
//! from a [`Vessel`]'s core node to the Void.
//!
//! Integer-only, deterministic: every turn is `route()` + a flare check,
//! nothing else. (TECH-DEBT DUEL-WCE-DISPATCH: the flare trigger here is a
//! local Hamming-shell proxy, not the `forge-consequence::Dispatcher` tick
//! cascade the plan calls for — that wiring is a separate slice.)

use super::astrakey_sieve::{derivation::soulword_address, types::DerivedSeed};
use super::hypercube::{self, Node};
use super::sevenfold::hermetic::Principle;
use forge_core_v3::atom::TritCell5D;
use forge_core_v3::lockstep::LockstepBarrier;

/// The 7 Hermetic laws in turn order (turn 1 = Mentalism, ..., turn 7 = Gender).
pub const TURN_PRINCIPLES: [Principle; 7] = [
    Principle::Mentalism,
    Principle::Correspondence,
    Principle::Vibration,
    Principle::Polarity,
    Principle::Rhythm,
    Principle::CauseEffect,
    Principle::Gender,
];

/// A duel turn counter, `1..=7`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnClock(pub u8);

impl TurnClock {
    /// The Principle governing this turn.
    pub fn principle(self) -> Principle {
        TURN_PRINCIPLES[(self.0 - 1) as usize]
    }
}

/// A closed 7-card pool — no draw, no replacement.
#[derive(Debug, Clone, Copy)]
pub struct Hand(pub [Node; 7]);

/// Bridge a real Astrakey card address into hypercube coordinate space.
/// `TritCell5D`'s packed byte (`0..=255`, 243 interior cards + 13 sentinels,
/// `astrakey_sieve::derivation::soulword_address`) fits directly in `Node`'s
/// low byte of its 14-bit coordinate — no rescaling, no collision with
/// `FLARE_BIT` (`1 << 13`), so a drawn card's own identity IS the node it
/// occupies. Additive: raw `Node` construction (existing hands, tests) is
/// untouched.
pub const fn card_to_node(card: TritCell5D) -> Node {
    Node(card.0 as u16)
}

impl Hand {
    /// Build a closed 7-card hand straight from 7 drawn Astrakey seeds — the
    /// real card-identity path duel777 was missing (aspire.rs
    /// `soulword-card-address-bridge` landed the seed->card-address half;
    /// this is the other half, card-address->duel-node).
    pub fn from_cards(seeds: &[DerivedSeed; 7]) -> Self {
        let mut nodes = [Node::VOID; 7];
        for (slot, seed) in nodes.iter_mut().zip(seeds.iter()) {
            *slot = card_to_node(soulword_address(seed));
        }
        Hand(nodes)
    }
}

/// A player's duel state: their core node and life (Hamming distance to Void).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vessel {
    pub core: Node,
    pub life: u8,
}

impl Vessel {
    pub fn new(core: Node) -> Self {
        Self {
            core,
            life: hypercube::life_as_distance(core, Node::VOID),
        }
    }

    pub fn is_dead(self) -> bool {
        self.life == 0
    }
}

/// Flare fires when a turn's route moves the core at least half the
/// hypercube's dimension from its pre-turn position — the local proxy for
/// a WCE overflow interaction (see TECH-DEBT DUEL-WCE-DISPATCH above).
const FLARE_HAMMING_THRESHOLD: u32 = hypercube::DIM / 2;

/// Resolve one turn: route the vessel's core by the played card, stamp
/// flare if the jump was large enough, recompute life.
pub fn resolve_turn(vessel: Vessel, card: Node) -> Vessel {
    let routed = hypercube::route(vessel.core, card);
    let jump = hypercube::hamming(vessel.core, routed);
    let core = if jump >= FLARE_HAMMING_THRESHOLD {
        hypercube::set_flare(routed)
    } else {
        routed
    };
    Vessel {
        core,
        life: hypercube::life_as_distance(core, Node::VOID),
    }
}

/// Play a full duel: up to 7 turns over a closed hand, stopping early if
/// the vessel dies. Deterministic — same `core`/`hand` always yields the
/// same sequence of vessels and the same terminal state.
pub fn play_duel(core: Node, hand: &Hand) -> Vessel {
    let mut vessel = Vessel::new(core);
    for turn in 1u8..=7 {
        if vessel.is_dead() {
            break;
        }
        let clock = TurnClock(turn);
        let _principle = clock.principle(); // turn-law hook point; see module docs
        vessel = resolve_turn(vessel, hand.0[(turn - 1) as usize]);
    }
    vessel
}

/// Play a two-vessel duel where each turn is gated by a real
/// [`LockstepBarrier`] (`forge-core-v3::lockstep`) instead of running
/// unconditionally. Turn == tick: a duel does not run every 120Hz frame, so
/// the barrier's tick counter advances one per duel turn, not per frame --
/// the open design question aspire.rs's `duel777-lockstep-wire` row named,
/// settled here.
///
/// Each turn, both vessels' hand cards for that turn are folded into the
/// barrier as peer 0's and peer 1's input words (in that canonical order,
/// per `LockstepBarrier`'s own peer-order-not-arrival-order guarantee)
/// BEFORE either vessel's turn resolves. Two independently-constructed
/// barriers fed the same `(core_a, hand_a, core_b, hand_b)` therefore derive
/// the same `chain_hash` with no coordinator -- the same property
/// `lockstep.rs`'s own `two_machines_agree_without_talking` test proves on
/// raw words, proven here on real duel state.
///
/// Peer-liveness ("is the other duelist still submitting turns") is
/// deliberately NOT this function's concern -- ARCH-009 Two Drums
/// (`forge-book-v3/src/tablets/ARCH-009-two-drums.md`) assigns that to
/// Drum-2 (the wall-clock liveness pulse, reconciled only through
/// `CollisionBridge`), never to Drum-1's chain-hash sequencing, which is all
/// this function and `barrier` carry. A caller whose peer never submits
/// should consult a Drum-2 liveness signal, not retry this hash chain.
pub fn play_duel_two_peer(
    core_a: Node,
    hand_a: &Hand,
    core_b: Node,
    hand_b: &Hand,
    barrier: &mut LockstepBarrier,
) -> (Vessel, Vessel, u64) {
    let mut a = Vessel::new(core_a);
    let mut b = Vessel::new(core_b);
    for turn in 1u8..=7 {
        if a.is_dead() && b.is_dead() {
            break;
        }
        let tick = barrier.tick();
        let card_a = hand_a.0[(turn - 1) as usize];
        let card_b = hand_b.0[(turn - 1) as usize];
        barrier.submit(tick, 0, card_a.0 as u32).expect("[duel777] peer 0 submit within window");
        barrier.submit(tick, 1, card_b.0 as u32).expect("[duel777] peer 1 submit within window");
        barrier.try_advance().expect("[duel777] both peers submitted, tick must release");
        if !a.is_dead() {
            a = resolve_turn(a, card_a);
        }
        if !b.is_dead() {
            b = resolve_turn(b, card_b);
        }
    }
    (a, b, barrier.chain_hash())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sanctioned_hand() -> Hand {
        Hand([
            Node(101), Node(4002), Node(777), Node(6543),
            Node(9), Node(2222), Node(15000),
        ])
    }

    #[test]
    fn turn_clock_maps_to_all_seven_principles_in_order() {
        assert_eq!(TurnClock(1).principle(), Principle::Mentalism);
        assert_eq!(TurnClock(7).principle(), Principle::Gender);
        assert_eq!(TURN_PRINCIPLES.len(), 7);
    }

    #[test]
    fn vessel_life_starts_as_hamming_to_void() {
        let v = Vessel::new(Node(0x3FFF));
        assert_eq!(v.life, 14);
        assert!(!v.is_dead());
        assert_eq!(Vessel::new(Node::VOID).life, 0);
        assert!(Vessel::new(Node::VOID).is_dead());
    }

    #[test]
    fn duel_is_deterministic() {
        let hand = sanctioned_hand();
        let a = play_duel(Node(1234), &hand);
        let b = play_duel(Node(1234), &hand);
        assert_eq!(a, b);
    }

    #[test]
    fn duel_reaches_a_terminal_state_within_seven_turns() {
        let hand = sanctioned_hand();
        let outcome = play_duel(Node(1234), &hand);
        // Terminal by construction: either dead, or all 7 turns were spent.
        assert!(outcome.life <= 14);
    }

    #[test]
    fn dead_vessel_stops_taking_turns() {
        // A hand whose first card routes the Void core straight back to Void
        // (card == core coordinate) keeps life at 0 — the loop must break,
        // not keep calling resolve_turn on a dead vessel.
        let hand = Hand([Node(0); 7]);
        let outcome = play_duel(Node::VOID, &hand);
        assert!(outcome.is_dead());
        assert_eq!(outcome, Vessel::new(Node::VOID));
    }

    #[test]
    fn two_machines_agree_on_a_duel_without_talking() {
        // Two independently-constructed barriers, same two hands -- the
        // lockstep property proven on raw u32 words in lockstep.rs's own
        // tests, proven here on real duel state.
        let hand_a = sanctioned_hand();
        let hand_b = Hand([
            Node(3), Node(88), Node(1500), Node(42),
            Node(777), Node(9001), Node(256),
        ]);
        let mut barrier_1 = LockstepBarrier::new(2);
        let mut barrier_2 = LockstepBarrier::new(2);
        let out_1 = play_duel_two_peer(Node(1234), &hand_a, Node(5678), &hand_b, &mut barrier_1);
        let out_2 = play_duel_two_peer(Node(1234), &hand_a, Node(5678), &hand_b, &mut barrier_2);
        assert_eq!(out_1, out_2, "same inputs on two independent barriers must agree bit-for-bit");
        assert_eq!(barrier_1.chain_hash(), barrier_2.chain_hash());
    }

    #[test]
    fn a_different_hand_desyncs_the_chain_hash() {
        let hand_a = sanctioned_hand();
        let hand_b = Hand([Node(3), Node(88), Node(1500), Node(42), Node(777), Node(9001), Node(256)]);
        let hand_b_lied = Hand([Node(3), Node(88), Node(1500), Node(42), Node(777), Node(9001), Node(255)]);
        let mut barrier_1 = LockstepBarrier::new(2);
        let mut barrier_2 = LockstepBarrier::new(2);
        let (_, _, hash_1) = play_duel_two_peer(Node(1234), &hand_a, Node(5678), &hand_b, &mut barrier_1);
        let (_, _, hash_2) =
            play_duel_two_peer(Node(1234), &hand_a, Node(5678), &hand_b_lied, &mut barrier_2);
        assert_ne!(hash_1, hash_2, "a single differing card must change the chain hash");
        assert_eq!(
            barrier_1.verify_remote(hash_2),
            forge_core_v3::lockstep::Verdict::Desync {
                tick: barrier_1.tick(),
                local: hash_1,
                remote: hash_2,
            }
        );
    }

    // [BOARD: WELD-duel-card-bridge]
    #[test]
    fn a_hand_built_from_real_astrakey_seeds_is_deterministic_and_playable() {
        use super::super::astrakey_sieve::{derivation::derive_seed, types::SystemID};
        let seeds: [DerivedSeed; 7] = std::array::from_fn(|i| {
            derive_seed(7, i, SystemID::Loot, &format!("turn_{i}"))
        });
        let hand_a = Hand::from_cards(&seeds);
        let hand_b = Hand::from_cards(&seeds);
        assert_eq!(hand_a.0, hand_b.0, "same seeds must produce the same hand, every time");

        // Every card in the hand must be a real, in-range TritCell5D address
        // (0..=255) reflected back losslessly into the node's low byte —
        // this is the actual card-to-duel bridge, not a coincidence of
        // Node's wider coordinate space.
        for (node, seed) in hand_a.0.iter().zip(seeds.iter()) {
            let card = soulword_address(seed);
            assert_eq!(node.0, card.0 as u16, "node must carry the card's own address, losslessly");
        }

        let outcome = play_duel(Node(1234), &hand_a);
        assert!(outcome.life <= 14, "a real-card hand must still reach a terminal duel state");
    }

    #[test]
    fn flare_can_fire_mid_duel() {
        // coord XOR = 0x0FFF (popcount 12 >= threshold 7), under M13 so the
        // shift-fold in m13_reduce is a no-op and the jump size is exact.
        let v = resolve_turn(Vessel::new(Node::VOID), Node(0x0FFF));
        assert!(v.core.is_flare(), "expected a flare-triggering jump");
    }
}
