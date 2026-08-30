//! What one party feels toward another, as a second-kind field. Five channels
//! that drive each other, resolved through [`Field5D`] into a [`Triad`] whose
//! disposition is the `Ta` axis [`super::trit_grammar`] renders from.

use forge_core_v3::cdk::Triad;
use forge_core_v3::decay::{LeakyPermyriad, PMY};
use forge_core_v3::resolvent::{macaulay_pow, Field5D};

use crate::ironroot::trit_grammar::{quantize_disposition, Q15};

/// Channels in a fixed order — the index into every `[i64; CHANNELS]` here and
/// into the coupling's rows and columns.
pub const CHANNELS: usize = 6;

/// Neumann iterations allowed before a field is called unsettled. A convergent
/// coupling settles in a handful; the cap exists so a defect surfaces as `None`
/// rather than a silent best-effort.
pub const MAX_ITERS: u32 = 64;

/// One thing a party can feel toward another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AffectChannel {
    /// Belief the other will do as they said.
    Trust,
    /// Expectation of harm.
    Fear,
    /// What is owed outward, unpaid.
    Debt,
    /// Harm taken and not yet answered.
    Grievance,
    /// What is owed inward, unclaimed.
    Obligation,
    /// What is not known about the other. Neutral and fillable — until it
    /// crosses into denial (see [`CROSSINGS`]).
    Ignorance,
}

impl AffectChannel {
    /// Every channel, in coupling order.
    pub const ALL: [AffectChannel; CHANNELS] = [
        AffectChannel::Trust,
        AffectChannel::Fear,
        AffectChannel::Debt,
        AffectChannel::Grievance,
        AffectChannel::Obligation,
        AffectChannel::Ignorance,
    ];

    /// Index into a channel vector.
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Name, for a ledger row or a test message.
    pub const fn name(self) -> &'static str {
        match self {
            AffectChannel::Trust => "trust",
            AffectChannel::Fear => "fear",
            AffectChannel::Debt => "debt",
            AffectChannel::Grievance => "grievance",
            AffectChannel::Obligation => "obligation",
            AffectChannel::Ignorance => "ignorance",
        }
    }
}

/// Per-channel memory. Each channel is a leaky integrator: an event injects,
/// every tick leaks. A slight suffered once fades; a slight suffered weekly
/// settles at [`LeakyPermyriad::equilibrium`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AffectMemory {
    channels: [LeakyPermyriad; CHANNELS],
}

impl AffectMemory {
    /// A memory with every channel clean and the given per-channel leaks.
    /// `None` if any leak is refused by [`LeakyPermyriad::new`] — a channel
    /// that never fades has no equilibrium and would accumulate without bound.
    pub fn new(leaks: [u16; CHANNELS]) -> Option<Self> {
        let mut channels = [LeakyPermyriad { value: 0, leak: 1, channel_rates: [0; 32] }; CHANNELS];
        let mut i = 0;
        while i < CHANNELS {
            match LeakyPermyriad::new(0, leaks[i]) {
                Some(c) => channels[i] = c,
                None => return None,
            }
            i += 1;
        }
        Some(Self { channels })
    }

    /// Leaks chosen so grievance outlasts fear and debt outlasts both. Authored
    /// against the shape of the thing, NOT measured — a grudge should decay
    /// slower than a fright. Replace when real play data exists.
    pub fn house() -> Self {
        // trust, fear, debt, grievance, obligation, ignorance
        // Ignorance leaks slowest of all: not-knowing does not fade on its own,
        // it is only displaced by being told.
        Self::new([300, 900, 120, 200, 150, 80]).expect("house leaks are all in 1..=PMY")
    }

    /// The same leaks, buoyed by a standing history.
    ///
    /// The leak IS gravity here: every channel falls toward zero every tick and
    /// nothing holds position for free — the reading `Triad::disposition`
    /// already gives entropy, that a bond never breaks even by standing still.
    /// Injection is work done against that fall, not lift.
    ///
    /// Buoyancy is the lift, and it scales with accumulated history the way real
    /// buoyancy scales with displaced volume: an old bond falls slower than a
    /// new one. `hops` is [`forge_core_v3::soul::cynatic_depth`]'s count to the
    /// root — the same history `Tn` reads — so a deep lineage is a deep hull.
    /// `None` (a broken or unbounded chain) buoys maximally: an unterminated
    /// history is not the same fact as no history.
    pub fn buoyed(leaks: [u16; CHANNELS], hops: Option<u32>) -> Option<Self> {
        let mut out = leaks;
        for l in &mut out {
            *l = buoyed_leak(*l, hops);
        }
        Self::new(out)
    }

    /// Add to one channel. Saturating, never wrapping.
    pub fn inject(&mut self, channel: AffectChannel, amount: u64) {
        self.channels[channel.index()].inject(amount);
    }

    /// One tick of forgetting across every channel.
    pub fn tick(&mut self) {
        for c in &mut self.channels {
            c.tick();
        }
    }

    /// The current raw value of one channel, before coupling.
    pub fn raw(&self, channel: AffectChannel) -> u64 {
        self.channels[channel.index()].value
    }

    /// The whole raw vector — the drive `g` the coupling resolves.
    pub fn drive(&self) -> [i64; CHANNELS] {
        let mut g = [0i64; CHANNELS];
        for (i, c) in self.channels.iter().enumerate() {
            g[i] = c.value.min(i64::MAX as u64) as i64;
        }
        g
    }

    /// Where one channel settles under constant injection at `rate`.
    pub fn equilibrium_of(&self, channel: AffectChannel, rate: u64) -> u64 {
        LeakyPermyriad::equilibrium(rate, self.channels[channel.index()].leak)
    }
}

/// How the channels drive each other. `m[i][j]` is parts-per-myriad of channel
/// `j` folded into channel `i`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AffectField {
    field: Field5D<CHANNELS>,
}

/// The authored coupling. Rows and columns run in [`AffectChannel::ALL`] order.
/// Diagonals are zero — a channel does not fold into itself, that is what the
/// identity in `(I − M)` already is.
///
/// THESE NUMBERS ARE AUTHORED, NOT MEASURED. They encode the shape claimed in
/// each comment and nothing stronger. Every row's absolute sum stays under
/// `PMY`, which is what makes the field convergent at all.
pub const HOUSE_COUPLING: [[i64; CHANNELS]; CHANNELS] = [
    // trust <- [trust, fear, debt, grievance, obligation, ignorance]
    // Fear and grievance eat trust; being owed to builds a little; you cannot
    // trust what you do not know.
    [0, -2500, 0, -2000, 1000, -1500],
    // fear <- ...
    // A standing grievance breeds fear of reprisal; trust damps it; the unknown
    // frightens on its own account.
    [-1500, 0, 0, 2000, 0, 2000],
    // debt <- ...
    // What you are owed makes you readier to owe.
    [0, 0, 0, 0, 2000, 0],
    // grievance <- ...
    // An unpaid debt sours into grievance; fear sharpens it; misreading someone
    // manufactures slights that were never given.
    [0, 1500, 2500, 0, 0, 1000],
    // obligation <- ...
    // Trust and debt both bind you inward.
    [2000, 0, 1500, 0, 0, 0],
    // ignorance <- ...
    // Trust makes you look and learn; fear stops you looking.
    [-1000, 1500, 0, 0, 0, 0],
];

impl AffectField {
    /// Build from a coupling, refusing one that will not settle
    /// ([`Field5D::new`] rejects any row summing to `PMY` or more).
    pub fn new(m: [[i64; CHANNELS]; CHANNELS]) -> Option<Self> {
        Field5D::new(m).map(|field| Self { field })
    }

    /// The authored coupling above.
    pub fn house() -> Self {
        Self::new(HOUSE_COUPLING).expect("HOUSE_COUPLING rows stay under PMY")
    }

    /// Resolve a drive into the settled field, `f = (I − M)⁻¹ g`. `None` if it
    /// did not settle within [`MAX_ITERS`].
    pub fn resolve(&self, drive: &[i64; CHANNELS]) -> Option<[i64; CHANNELS]> {
        self.field.resolve(drive, MAX_ITERS)
    }

    /// Recover the drive a settled field came from — the exact inverse.
    pub fn deproject(&self, settled: &[i64; CHANNELS]) -> [i64; CHANNELS] {
        self.field.deproject(settled)
    }

    /// Resolve a memory straight to its settled field.
    pub fn settle(&self, memory: &AffectMemory) -> Option<[i64; CHANNELS]> {
        self.resolve(&memory.drive())
    }
}

/// Hop count past which extra history buys no further lift. A hull only
/// displaces so much.
pub const BUOYANCY_CAP_HOPS: u32 = 8;

/// Divide a leak by the history standing under it, never below 1 — a channel
/// that stopped leaking entirely would have no equilibrium and
/// [`LeakyPermyriad::new`] refuses it. Buoyancy slows the fall; it never
/// cancels it.
pub fn buoyed_leak(base_leak: u16, hops: Option<u32>) -> u16 {
    let depth = hops.unwrap_or(BUOYANCY_CAP_HOPS).min(BUOYANCY_CAP_HOPS);
    let divisor = 1 + depth as u32;
    ((base_leak as u32 / divisor).max(1)) as u16
}

/// A channel changing KIND, not degree.
///
/// Some feelings scale smoothly and stay answerable to their own remedy. Others
/// have a threshold past which they become a different thing and go deaf to it:
/// fear you can reassure, terror you cannot; ignorance you can inform, denial
/// you cannot — telling a denier only digs them in.
///
/// That is one law, not a pile of special cases, and its shape is the Macaulay
/// bracket `⟨x − onset⟩ⁿ`: exactly zero below the threshold, accelerating above
/// it. Below onset the crossed state does not exist at all — terror is not a
/// small dose of fear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Crossing {
    /// The channel that crosses.
    pub channel: AffectChannel,
    /// What it becomes.
    pub name: &'static str,
    /// What stops working once crossed — the remedy it goes deaf to.
    pub deaf_to: &'static str,
    /// Q15 SHARE of the settled field's magnitude below which the crossed
    /// state is zero — the channel's direction cosine against its own axis,
    /// the same read [`quantize_disposition`] makes against the disposition
    /// pole. A share is bond state; it does not move when the drive scales.
    pub onset: i64,
    /// Power it climbs by above the onset. `2` accelerates rather than ramps.
    pub order: u32,
}

/// Field magnitude below which no crossing exists at all: a bond too faint to
/// feel has no composition to cross on. The one absolute constant left, and it
/// gates existence, not kind.
pub const FELT_FLOOR: i64 = 1_000;

/// Euclidean magnitude of a settled field — the denominator every share read
/// divides by.
pub fn field_magnitude(resolved: &[i64; CHANNELS]) -> i64 {
    let mut sum: u128 = 0;
    for v in resolved {
        sum += (v * v) as u128;
    }
    sum.isqrt() as i64
}

impl Crossing {
    /// Macaulay bracket on the channel's Q15 share past `onset`, scaled back to
    /// field units by the magnitude. Exactly zero below onset or below
    /// [`FELT_FLOOR`].
    pub fn amount(&self, resolved: &[i64; CHANNELS]) -> i64 {
        let mag = field_magnitude(resolved);
        if mag < FELT_FLOOR {
            return 0;
        }
        let share = resolved[self.channel.index()] * Q15 / mag;
        macaulay_pow(share, self.onset, self.order) * mag / (Q15 * Q15)
    }

    /// Whether this crossing has happened.
    pub fn crossed(&self, resolved: &[i64; CHANNELS]) -> bool {
        self.amount(resolved) > 0
    }
}

/// The crossings this model carries. Onsets are Q15 shares — a bond crosses
/// when the channel DOMINATES its composition, at any scale. Orders stay
/// authored; onset values are held to the census gates in the tests below.
pub const CROSSINGS: [Crossing; 2] = [
    Crossing {
        channel: AffectChannel::Fear,
        name: "terror",
        deaf_to: "reassurance",
        onset: 30_100,
        order: 2,
    },
    Crossing {
        channel: AffectChannel::Ignorance,
        name: "denial",
        deaf_to: "information",
        onset: 30_300,
        order: 2,
    },
];

/// Every crossing currently in force, with how far past its threshold it is.
pub fn crossings_in_force(resolved: &[i64; CHANNELS]) -> Vec<(&'static str, i64)> {
    CROSSINGS
        .iter()
        .filter(|c| c.crossed(resolved))
        .map(|c| (c.name, c.amount(resolved)))
        .collect()
}

/// Total weight of every crossed state. A crossed channel has stopped being
/// negotiable, so this rides entropy in [`to_triad`] rather than strife: it is
/// a cut neither love nor strife can bargain with.
pub fn crossed_weight(resolved: &[i64; CHANNELS]) -> i64 {
    CROSSINGS.iter().map(|c| c.amount(resolved)).sum()
}

/// One prose claim a nonzero coupling entry makes, as a checkable fact.
/// `raises` is the sign of `m[of][by]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CouplingClaim {
    /// Row — the channel that moves.
    pub of: AffectChannel,
    /// Column — the channel that moves it.
    pub by: AffectChannel,
    /// The claim in words, so a sign change cannot pass as a typo.
    pub claim: &'static str,
    /// True where the authored entry is positive.
    pub raises: bool,
}

/// Every nonzero entry of [`HOUSE_COUPLING`], named. The magnitudes stay
/// authored; the SHAPE is pinned here and gated by
/// `the_claim_table_and_the_matrix_are_the_same_fact`.
pub const COUPLING_CLAIMS: [CouplingClaim; 15] = [
    CouplingClaim { of: AffectChannel::Trust, by: AffectChannel::Fear, claim: "fear eats trust", raises: false },
    CouplingClaim { of: AffectChannel::Trust, by: AffectChannel::Grievance, claim: "a standing grievance eats trust", raises: false },
    CouplingClaim { of: AffectChannel::Trust, by: AffectChannel::Obligation, claim: "being owed to builds a little trust", raises: true },
    CouplingClaim { of: AffectChannel::Trust, by: AffectChannel::Ignorance, claim: "you cannot trust what you do not know", raises: false },
    CouplingClaim { of: AffectChannel::Fear, by: AffectChannel::Trust, claim: "trust damps fear", raises: false },
    CouplingClaim { of: AffectChannel::Fear, by: AffectChannel::Grievance, claim: "a grievance breeds fear of reprisal", raises: true },
    CouplingClaim { of: AffectChannel::Fear, by: AffectChannel::Ignorance, claim: "the unknown frightens on its own account", raises: true },
    CouplingClaim { of: AffectChannel::Debt, by: AffectChannel::Obligation, claim: "what you are owed makes you readier to owe", raises: true },
    CouplingClaim { of: AffectChannel::Grievance, by: AffectChannel::Fear, claim: "fear sharpens a grievance", raises: true },
    CouplingClaim { of: AffectChannel::Grievance, by: AffectChannel::Debt, claim: "an unpaid debt sours into grievance", raises: true },
    CouplingClaim { of: AffectChannel::Grievance, by: AffectChannel::Ignorance, claim: "misreading someone manufactures slights never given", raises: true },
    CouplingClaim { of: AffectChannel::Obligation, by: AffectChannel::Trust, claim: "trust binds you inward", raises: true },
    CouplingClaim { of: AffectChannel::Obligation, by: AffectChannel::Debt, claim: "debt binds you inward", raises: true },
    CouplingClaim { of: AffectChannel::Ignorance, by: AffectChannel::Trust, claim: "trust makes you look and learn", raises: false },
    CouplingClaim { of: AffectChannel::Ignorance, by: AffectChannel::Fear, claim: "fear stops you looking", raises: true },
];

/// Probe delta used when asking what one channel does to another through the
/// whole field. Large enough that truncation cannot decide the sign.
pub const PROBE_DELTA: i64 = 10_000;

/// How much resolved `of` moves when `by`'s drive rises by [`PROBE_DELTA`] —
/// the settled answer, not the raw entry. `None` if either field fails to
/// settle.
pub fn resolved_response(field: &AffectField, of: AffectChannel, by: AffectChannel) -> Option<i64> {
    let base = [0i64; CHANNELS];
    let mut lifted = base;
    lifted[by.index()] = PROBE_DELTA;
    let a = field.resolve(&base)?;
    let b = field.resolve(&lifted)?;
    Some(b[of.index()] - a[of.index()])
}

/// Drives the sensitivity sweep probes. Fixed and small: a spread of one-hot,
/// warm, sour and mixed bonds.
pub const PROBE_DRIVES: [[i64; CHANNELS]; 9] = [
    [6_000, 0, 0, 0, 4_000, 0],
    [0, 5_000, 0, 6_000, 0, 0],
    [3_000, 0, 0, 4_000, 0, 0],
    [1_000, 3_000, 0, 9_000, 0, 0],
    [0, 14_000, 0, 0, 0, 6_000],
    [4_000, 0, 8_000, 0, 0, 0],
    [9_000, 0, 0, 0, 7_000, 0],
    [0, 0, 0, 0, 0, 11_000],
    [2_000, 2_000, 2_000, 2_000, 2_000, 2_000],
];

/// What a sweep of one authored entry did to the reading it drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntrySensitivity {
    /// Row of the swept entry.
    pub of: AffectChannel,
    /// Column of the swept entry.
    pub by: AffectChannel,
    /// The authored value.
    pub authored: i64,
    /// Probes whose `Ta` sign changed under some swept value.
    pub sign_flips: u32,
    /// Probes that stopped settling under some swept value.
    pub unsettled: u32,
    /// Swept values that the norm guard refused outright.
    pub refused: u32,
    /// Probes evaluated, per swept value.
    pub probes: u32,
    /// Largest absolute change in `disposition` any sweep produced, BEFORE the
    /// sign quantizer. Separates "this entry does nothing" from "the quantizer
    /// throws away what it does".
    pub disposition_span: i64,
}

/// Multipliers applied to one entry at a time, in permyriad of the authored
/// value. Halve, three-quarter, one-and-a-half, double, and negate.
const SWEEP: [i64; 5] = [5_000, 7_500, 15_000, 20_000, -10_000];

fn disposition_of(settled: &[i64; CHANNELS]) -> i64 {
    to_triad(settled, 0).disposition() as i64
}

fn ta_sign(settled: &[i64; CHANNELS]) -> i8 {
    quantize_disposition(&to_triad(settled, 0))
}

/// Sweep every authored entry one at a time and count how many probe readings
/// its value actually decides. An entry no probe reacts to is authored
/// precision that carries no information; an entry that flips signs is one
/// that needs real data before it can be trusted.
pub fn entry_sensitivity() -> Vec<EntrySensitivity> {
    let house = AffectField::house();
    let baseline: Vec<Option<(i8, i64)>> = PROBE_DRIVES
        .iter()
        .map(|g| house.resolve(g).map(|f| (ta_sign(&f), disposition_of(&f))))
        .collect();

    let mut out = Vec::with_capacity(COUPLING_CLAIMS.len());
    for c in COUPLING_CLAIMS {
        let (i, j) = (c.of.index(), c.by.index());
        let authored = HOUSE_COUPLING[i][j];
        let mut row = EntrySensitivity {
            of: c.of,
            by: c.by,
            authored,
            sign_flips: 0,
            unsettled: 0,
            refused: 0,
            probes: PROBE_DRIVES.len() as u32,
            disposition_span: 0,
        };
        for mul in SWEEP {
            let mut m = HOUSE_COUPLING;
            m[i][j] = authored * mul / PMY as i64;
            let Some(field) = AffectField::new(m) else {
                row.refused += 1;
                continue;
            };
            for (g, base) in PROBE_DRIVES.iter().zip(&baseline) {
                match (field.resolve(g), base) {
                    (Some(f), Some((sign, disp))) => {
                        if ta_sign(&f) != *sign {
                            row.sign_flips += 1;
                        }
                        row.disposition_span = row.disposition_span.max((disposition_of(&f) - disp).abs());
                    }
                    (None, Some(_)) => row.unsettled += 1,
                    _ => {}
                }
            }
        }
        out.push(row);
    }
    out
}

/// A whole authored model: the coupling AND the thresholds. Both are authored,
/// both are swept the same way, so they belong to one struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AffectModel {
    /// The coupling.
    pub field: AffectField,
    /// The thresholds at which channels change kind.
    pub crossings: [Crossing; 2],
}

impl AffectModel {
    /// The authored house model.
    pub fn house() -> Self {
        Self { field: AffectField::house(), crossings: CROSSINGS }
    }

    /// Total weight of every crossed state under THIS model's thresholds.
    fn crossed_weight(&self, settled: &[i64; CHANNELS]) -> i64 {
        self.crossings.iter().map(|c| c.amount(settled)).sum()
    }

    /// The triad a settled field folds to under THIS model's crossings.
    fn triad_of_settled(&self, f: &[i64; CHANNELS]) -> Triad {
        let at = |c: AffectChannel| f[c.index()];
        Triad {
            love: (at(AffectChannel::Trust) + at(AffectChannel::Obligation)) as i32,
            strife: (at(AffectChannel::Fear) + at(AffectChannel::Grievance)) as i32,
            entropy: (at(AffectChannel::Debt) + self.crossed_weight(f)) as i32,
        }
    }

    /// Resolve a drive to its triad, or `None` if the field did not settle.
    pub fn triad(&self, drive: &[i64; CHANNELS]) -> Option<Triad> {
        self.field.resolve(drive).map(|f| self.triad_of_settled(&f))
    }

    /// The settled disposition, or `None` if the field did not settle.
    pub fn disposition(&self, drive: &[i64; CHANNELS]) -> Option<i64> {
        self.triad(drive).map(|t| t.disposition() as i64)
    }

    /// The `Ta` trit this model reads from a drive — the only thing about a
    /// bond that reaches a player. The banded direction read, one home:
    /// [`quantize_disposition`].
    pub fn reading(&self, drive: &[i64; CHANNELS]) -> Option<i8> {
        self.triad(drive).map(|t| quantize_disposition(&t))
    }
}

/// Largest per-channel drive sampled. `AffectMemory` is unbounded in principle
/// (sustained injection at rate `r` on ignorance settles at `r·125`), so this is
/// an envelope, not a limit: it covers every `mudlex affect` scenario injection
/// (max 14_000) with room over.
pub const DRIVE_CEILING: i64 = 30_000;

/// How drives are drawn. The whole cube answers whether an entry matters
/// mathematically; sparse bonds answer whether it matters in play.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveRegime {
    /// All six channels uniform over `0..=DRIVE_CEILING`.
    Dense,
    /// One to three channels lit, the rest exactly zero — the shape a real
    /// bond has, and the shape every authored scenario in `mudlex affect` is.
    Sparse,
}

/// SplitMix64. Counter-based so a sample index maps to a drive with no carried
/// state: any range of indices can be drawn on any thread in any order and the
/// census is identical.
#[inline(always)]
const fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The drive for one sample index. Deterministic and stateless.
#[inline]
pub fn sample_drive(index: u64, regime: DriveRegime) -> [i64; CHANNELS] {
    let span = DRIVE_CEILING as u64 + 1;
    let mut g = [0i64; CHANNELS];
    match regime {
        DriveRegime::Dense => {
            for (k, slot) in g.iter_mut().enumerate() {
                *slot = (mix(index.wrapping_mul(CHANNELS as u64).wrapping_add(k as u64)) % span) as i64;
            }
        }
        DriveRegime::Sparse => {
            let h = mix(index);
            let lit = 1 + (h % 3) as usize;
            for k in 0..lit {
                let pick = mix(h.wrapping_add(k as u64 + 1));
                let ch = (pick % CHANNELS as u64) as usize;
                g[ch] = (mix(pick) % span) as i64;
            }
        }
    }
    g
}

/// What a run of samples found under one model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AffectCensus {
    /// Drives drawn.
    pub samples: u64,
    /// Readings of `+1` — the bond reads as held.
    pub bound: u64,
    /// Readings of `0` — exact balance.
    pub neutral: u64,
    /// Readings of `-1` — the bond reads as torn.
    pub torn: u64,
    /// Drives the field could not settle. The truncation cycle, counted.
    pub unsettled: u64,
    /// Settled drives past the terror onset.
    pub terror: u64,
    /// Settled drives past the denial onset.
    pub denial: u64,
    /// Extremes of settled disposition seen.
    pub disposition_min: i64,
    /// Extremes of settled disposition seen.
    pub disposition_max: i64,
}

impl AffectCensus {
    /// Fold two disjoint ranges into one. Associative, so thread order cannot
    /// change the answer.
    pub fn merge(self, other: Self) -> Self {
        Self {
            samples: self.samples + other.samples,
            bound: self.bound + other.bound,
            neutral: self.neutral + other.neutral,
            torn: self.torn + other.torn,
            unsettled: self.unsettled + other.unsettled,
            terror: self.terror + other.terror,
            denial: self.denial + other.denial,
            disposition_min: self.disposition_min.min(other.disposition_min),
            disposition_max: self.disposition_max.max(other.disposition_max),
        }
    }

    /// Parts per myriad of `n` out of the settled samples.
    pub fn permyriad(&self, n: u64) -> u64 {
        let settled = self.samples - self.unsettled;
        if settled == 0 { 0 } else { n * PMY / settled }
    }
}

/// AffectCensus one half-open range of sample indices under one model.
pub fn census_range(model: &AffectModel, regime: DriveRegime, lo: u64, hi: u64) -> AffectCensus {
    let mut c = AffectCensus { disposition_min: i64::MAX, disposition_max: i64::MIN, ..AffectCensus::default() };
    for i in lo..hi {
        c.samples += 1;
        let drive = sample_drive(i, regime);
        let Some(f) = model.field.resolve(&drive) else {
            c.unsettled += 1;
            continue;
        };
        if model.crossings[0].crossed(&f) {
            c.terror += 1;
        }
        if model.crossings[1].crossed(&f) {
            c.denial += 1;
        }
        let t = model.triad_of_settled(&f);
        let d = t.disposition() as i64;
        c.disposition_min = c.disposition_min.min(d);
        c.disposition_max = c.disposition_max.max(d);
        match quantize_disposition(&t) {
            1 => c.bound += 1,
            0 => c.neutral += 1,
            _ => c.torn += 1,
        }
    }
    c
}

/// How far two models disagree over the same drives — the question a single
/// authored constant actually poses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AffectDivergence {
    /// Drives drawn.
    pub samples: u64,
    /// Drives where both models settled.
    pub both_settled: u64,
    /// Drives where both settled and the `Ta` trit differs.
    pub ta_flips: u64,
    /// Drives one model settled and the other did not.
    pub settling_differs: u64,
    /// Largest absolute disposition gap where both settled.
    pub disposition_gap: i64,
}

impl AffectDivergence {
    /// Fold two disjoint ranges. Associative.
    pub fn merge(self, other: Self) -> Self {
        Self {
            samples: self.samples + other.samples,
            both_settled: self.both_settled + other.both_settled,
            ta_flips: self.ta_flips + other.ta_flips,
            settling_differs: self.settling_differs + other.settling_differs,
            disposition_gap: self.disposition_gap.max(other.disposition_gap),
        }
    }

    /// Flip rate in parts per myriad of the drives both models settled.
    pub fn flip_permyriad(&self) -> u64 {
        if self.both_settled == 0 { 0 } else { self.ta_flips * PMY / self.both_settled }
    }
}

/// Compare two models over one half-open range of sample indices.
pub fn compare_range(a: &AffectModel, b: &AffectModel, regime: DriveRegime, lo: u64, hi: u64) -> AffectDivergence {
    let mut d = AffectDivergence::default();
    for i in lo..hi {
        d.samples += 1;
        let drive = sample_drive(i, regime);
        match (a.triad(&drive), b.triad(&drive)) {
            (Some(x), Some(y)) => {
                d.both_settled += 1;
                if quantize_disposition(&x) != quantize_disposition(&y) {
                    d.ta_flips += 1;
                }
                d.disposition_gap =
                    d.disposition_gap.max((x.disposition() as i64 - y.disposition() as i64).abs());
            }
            (None, None) => {}
            _ => d.settling_differs += 1,
        }
    }
    d
}

/// The model with one coupling entry replaced. `None` if the norm guard refuses
/// the result.
pub fn variant_entry(of: AffectChannel, by: AffectChannel, value: i64) -> Option<AffectModel> {
    let mut m = HOUSE_COUPLING;
    m[of.index()][by.index()] = value;
    AffectField::new(m).map(|field| AffectModel { field, crossings: CROSSINGS })
}

/// The model with one crossing's onset replaced.
pub fn variant_onset(which: usize, onset: i64) -> AffectModel {
    let mut crossings = CROSSINGS;
    crossings[which].onset = onset;
    AffectModel { field: AffectField::house(), crossings }
}

/// Q15 share onsets swept by [`onset_sensitivity`], astride the calibrated
/// onsets near 30_000 (rate-bisected to ~5% of sparse drives).
const ONSET_SWEEP: [i64; 6] = [3_277, 9_830, 16_384, 22_937, 29_490, 32_767];

/// What sweeping one crossing's onset did to the readings it drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OnsetSensitivity {
    /// The crossing swept.
    pub name: &'static str,
    /// Its authored onset.
    pub authored: i64,
    /// Probes whose `Ta` sign changed under some swept onset.
    pub sign_flips: u32,
    /// Probes that crossed under some swept onset but not the authored one.
    pub crossing_changes: u32,
    /// Largest absolute change in `disposition` any sweep produced.
    pub disposition_span: i64,
    /// Probes evaluated, per swept onset.
    pub probes: u32,
}

/// Sweep each crossing's onset the same way [`entry_sensitivity`] sweeps a
/// coupling entry. A threshold nothing reacts to is not a threshold.
pub fn onset_sensitivity() -> Vec<OnsetSensitivity> {
    let house = AffectField::house();
    let settled: Vec<[i64; CHANNELS]> =
        PROBE_DRIVES.iter().filter_map(|g| house.resolve(g)).collect();

    CROSSINGS
        .iter()
        .map(|c| {
            let base: Vec<(i8, i64, bool)> = settled
                .iter()
                .map(|f| (ta_sign(f), disposition_of(f), c.crossed(f)))
                .collect();
            let mut row = OnsetSensitivity {
                name: c.name,
                authored: c.onset,
                sign_flips: 0,
                crossing_changes: 0,
                disposition_span: 0,
                probes: settled.len() as u32,
            };
            for onset in ONSET_SWEEP {
                if onset == c.onset {
                    continue;
                }
                let swept = Crossing { onset, ..*c };
                for (f, (sign, _disp, was)) in settled.iter().zip(&base) {
                    // Crossed weight lands in entropy, which disposition subtracts.
                    let shifted = swept.amount(f) - c.amount(f);
                    let base_triad = to_triad(f, 0);
                    let adjusted = Triad {
                        entropy: base_triad.entropy + shifted as i32,
                        ..base_triad
                    };
                    let s = quantize_disposition(&adjusted);
                    if s != *sign {
                        row.sign_flips += 1;
                    }
                    if swept.crossed(f) != *was {
                        row.crossing_changes += 1;
                    }
                    row.disposition_span = row.disposition_span.max(shifted.abs());
                }
            }
            row
        })
        .collect()
}

/// Fold a settled field into the Empedoclean triad.
///
/// LOVE binds: trust and obligation, the two that hold a tie together.
/// STRIFE separates: fear and grievance, the two that pull it apart.
/// ENTROPY is what neither holds — an unpaid debt sits outside the bond and
/// takes its cut regardless, which is the reading `Triad::disposition` already
/// gives entropy. `haunt` adds the room's own entropy on top, permyriad.
///
/// The channel split is the point: love and strife draw on DIFFERENT channels,
/// so disposition can land either side of zero. Deriving both from one pool is
/// what made two earlier hand-picked mappings collapse to a single sign.
pub fn to_triad(settled: &[i64; CHANNELS], haunt: u32) -> Triad {
    let at = |c: AffectChannel| settled[c.index()];
    Triad {
        love: (at(AffectChannel::Trust) + at(AffectChannel::Obligation)) as i32,
        strife: (at(AffectChannel::Fear) + at(AffectChannel::Grievance)) as i32,
        // Debt sits outside the bond and takes its cut. So does any crossed
        // state: terror and denial have stopped answering to anything either
        // party can offer, which is what entropy means here.
        entropy: (at(AffectChannel::Debt) + crossed_weight(settled)) as i32
            + haunt.min(Triad::HAUNT_MAX as u32) as i32,
    }
}

/// The whole path: memory -> coupling -> settled field -> triad. `None` if the
/// field did not settle.
pub fn triad_of(memory: &AffectMemory, field: &AffectField, haunt: u32) -> Option<Triad> {
    field.settle(memory).map(|f| to_triad(&f, haunt))
}

/// Largest absolute row sum of a coupling, in permyriad — the norm
/// [`Field5D::new`] gates on. Exposed so a caller authoring its own matrix can
/// see how much headroom it has before the field stops settling.
pub fn coupling_norm(m: &[[i64; CHANNELS]; CHANNELS]) -> i64 {
    let mut worst = 0;
    for row in m {
        let sum: i64 = row.iter().map(|v| v.abs()).sum();
        if sum > worst {
            worst = sum;
        }
    }
    worst
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ironroot::trit_grammar::quantize_disposition;

    /// A memory with the given channel values already standing.
    fn standing(values: [(AffectChannel, u64); 3]) -> AffectMemory {
        let mut m = AffectMemory::house();
        for (c, v) in values {
            m.inject(c, v);
        }
        m
    }

    #[test]
    fn the_house_coupling_converges_and_a_runaway_is_refused() {
        assert!(coupling_norm(&HOUSE_COUPLING) < PMY as i64, "house must leave headroom");
        assert!(AffectField::new(HOUSE_COUPLING).is_some());

        // Every row at PMY/2 across 5 channels sums to 2.5x PMY — no equilibrium.
        let runaway = [[(PMY / 2) as i64; CHANNELS]; CHANNELS];
        assert!(coupling_norm(&runaway) >= PMY as i64);
        assert!(
            AffectField::new(runaway).is_none(),
            "a coupling with no bounded equilibrium must refuse, not settle badly"
        );
    }

    #[test]
    fn the_coupling_is_a_bijection_between_drive_and_settled_field() {
        // L07: deproject is the exact inverse of resolve. If this ever fails the
        // settled field is not recoverable and the model has lost information.
        let field = AffectField::house();
        let mut checked = 0;
        for drive in [
            [1000i64, 0, 0, 0, 0, 0],
            [0, 2000, 500, 3000, 0, 0],
            [3000, 0, 0, 4000, 0, 0],
            [6000, 0, 0, 0, 4000, 0],
            [0, 0, 0, 0, 0, 5000],
        ] {
            // A drive that does not settle is a separate, pinned fact; the
            // bijection claim only binds where a settled field exists.
            if let Some(settled) = field.resolve(&drive) {
                assert_eq!(field.deproject(&settled), drive, "deproject(resolve(g)) must be g");
                checked += 1;
            }
        }
        assert!(checked >= 3, "the bijection must hold on real settled fields, got {checked}");
    }

    // A KNOWN BOUNDARY of the kernel primitive. `Field5D::new` admits a coupling
    // on its infinity norm, which guarantees convergence in REAL arithmetic.
    // `scale` truncates toward zero, so with mixed-sign entries a dense drive can
    // land in a period-2 cycle whose two iterates differ by 1 and never reach an
    // exact integer fixed point. Measured on the 5-channel draft of this matrix
    // (2026-08-25): drive [4000, 1000, 2000, 1500, 800] cycled between
    // [3510, 919, 2371, 2230, 1857] and [3510, 920, 2371, 2229, 1857], still
    // cycling at 100_000 iterations.
    //
    // `resolve` returns None rather than guessing, which is the contract working.
    // This test does NOT assert a specific non-settling vector — that would pin a
    // number to one matrix. It asserts the honest property: whatever settles is
    // exactly invertible, and whatever does not is reported as None.
    #[test]
    fn a_field_either_settles_exactly_or_says_it_did_not() {
        let field = AffectField::house();
        assert!(coupling_norm(&HOUSE_COUPLING) < PMY as i64, "house is norm-admissible");

        let mut settled = 0;
        let mut unsettled = 0;
        for a in [0i64, 2_500, 6_000] {
            for b in [0i64, 1_500, 4_000] {
                for c in [0i64, 3_000] {
                    let drive = [a, b, c, b, a, c];
                    match field.resolve(&drive) {
                        Some(f) => {
                            assert_eq!(field.deproject(&f), drive, "a settled field must invert exactly");
                            settled += 1;
                        }
                        None => unsettled += 1,
                    }
                }
            }
        }
        assert!(settled > 0, "the house coupling must settle for some real drives");
        eprintln!("affect: {settled} settled, {unsettled} reported unsettled");
    }

    #[test]
    fn grievance_eats_trust_through_the_coupling() {
        let field = AffectField::house();
        let clean = field.resolve(&[3000, 0, 0, 0, 0, 0]).expect("settles");
        let soured = field.resolve(&[3000, 0, 0, 4000, 0, 0]).expect("settles");
        let i = AffectChannel::Trust.index();
        assert!(
            soured[i] < clean[i],
            "a standing grievance must lower resolved trust: {} vs {}",
            soured[i],
            clean[i]
        );
        // And it does so WITHOUT the drive changing — the raw trust injected is
        // identical in both. The drop is second-order, which is the whole point.
        assert_eq!(field.deproject(&soured)[i], 3000);
    }

    #[test]
    fn a_slight_suffered_once_fades_but_a_habit_settles() {
        let mut once = AffectMemory::house();
        once.inject(AffectChannel::Grievance, 5_000);
        let start = once.raw(AffectChannel::Grievance);
        for _ in 0..200 {
            once.tick();
        }
        assert!(once.raw(AffectChannel::Grievance) < start / 10, "one slight fades");

        let rate = 100;
        let mut habit = AffectMemory::house();
        for _ in 0..500 {
            habit.inject(AffectChannel::Grievance, rate);
            habit.tick();
        }
        let settled = habit.raw(AffectChannel::Grievance);
        let closed_form = habit.equilibrium_of(AffectChannel::Grievance, rate);
        assert!(settled > 0, "a repeated slight does not fade to nothing");
        assert!(
            settled <= closed_form,
            "the flooring engine settles at or just below the closed form: {settled} vs {closed_form}"
        );
        assert!(
            settled * 2 > closed_form,
            "and it should be near it, not merely below: {settled} vs {closed_form}"
        );
    }

    #[test]
    fn a_memory_channel_that_never_fades_is_refused() {
        assert!(AffectMemory::new([300, 900, 120, 0, 150, 80]).is_none(), "leak 0 has no equilibrium");
        assert!(AffectMemory::new([300, 900, 120, (PMY + 1) as u16, 150, 80]).is_none());
    }

    #[test]
    fn history_buoys_a_bond_against_the_leak_but_never_cancels_it() {
        // Gravity is the leak. Buoyancy is history. A deep hull falls slower.
        let shallow = buoyed_leak(900, Some(0));
        let mid = buoyed_leak(900, Some(4));
        let deep = buoyed_leak(900, Some(BUOYANCY_CAP_HOPS));
        assert!(deep < mid && mid < shallow, "{shallow} {mid} {deep}");
        assert_eq!(shallow, 900, "no history, no lift — the raw leak stands");

        // Past the cap a hull displaces no more.
        assert_eq!(buoyed_leak(900, Some(BUOYANCY_CAP_HOPS + 50)), deep);
        // A broken chain buoys maximally, never neutrally.
        assert_eq!(buoyed_leak(900, None), deep);

        // It slows the fall; it never stops it. A leak of 0 has no equilibrium
        // and LeakyPermyriad would refuse it outright.
        assert!(buoyed_leak(1, Some(BUOYANCY_CAP_HOPS)) >= 1);
        assert!(AffectMemory::buoyed([300, 900, 120, 200, 150, 80], None).is_some());
    }

    #[test]
    fn a_deep_bond_outlasts_a_new_one_from_the_same_wound() {
        let mut new_bond = AffectMemory::buoyed([300, 900, 120, 200, 150, 80], Some(0)).expect("valid");
        let mut old_bond = AffectMemory::buoyed([300, 900, 120, 200, 150, 80], Some(BUOYANCY_CAP_HOPS)).expect("valid");
        new_bond.inject(AffectChannel::Trust, 8_000);
        old_bond.inject(AffectChannel::Trust, 8_000);
        for _ in 0..50 {
            new_bond.tick();
            old_bond.tick();
        }
        assert!(
            old_bond.raw(AffectChannel::Trust) > new_bond.raw(AffectChannel::Trust),
            "the same goodwill must last longer where there is history under it: {} vs {}",
            old_bond.raw(AffectChannel::Trust),
            new_bond.raw(AffectChannel::Trust)
        );
    }

    #[test]
    fn a_crossing_is_exactly_zero_below_its_onset_share_and_accelerates_above() {
        let terror = CROSSINGS[0];
        assert_eq!(terror.channel, AffectChannel::Fear);
        assert_eq!(terror.name, "terror");

        let mix = |fear: i64, trust: i64| {
            let mut f = [0i64; CHANNELS];
            f[AffectChannel::Fear.index()] = fear;
            f[AffectChannel::Trust.index()] = trust;
            f
        };
        assert_eq!(terror.amount(&mix(0, 10_000)), 0, "no fear, no terror");
        assert_eq!(terror.amount(&mix(3_000, 10_000)), 0, "a minor note of fear is not terror");
        assert_eq!(terror.amount(&mix(9_000, 4_359)), 0, "even a strongly fear-led bond (share 0.90) is below the calibrated onset");
        assert!(terror.amount(&mix(9_950, 998)) > 0, "a bond that is almost nothing but fear is past onset");
        assert_eq!(terror.amount(&mix(900, 100)), 0, "below the felt floor there is no bond to cross");

        // Accelerating, not ramping, in SHARE at fixed magnitude 10_000:
        // (9_400, 3_412) is share ~0.94, (9_800, 1_990) is share ~0.98 —
        // near triple the overshoot past the 0.918 onset, far more weight.
        let one = terror.amount(&mix(9_400, 3_412));
        let two = terror.amount(&mix(9_800, 1_990));
        assert!(one > 0, "share 0.94 is past the calibrated onset");
        assert!(two > one * 3, "order 2 accelerates in share: {one} then {two}");
    }

    #[test]
    fn every_crossing_goes_deaf_to_its_own_remedy() {
        // The law both crossings encode. Named here so the pair cannot drift
        // apart into two unrelated special cases.
        let names: Vec<_> = CROSSINGS.iter().map(|c| (c.name, c.deaf_to)).collect();
        assert_eq!(names, [("terror", "reassurance"), ("denial", "information")]);
        for c in CROSSINGS {
            assert!(c.order >= 2, "{} must accelerate, not ramp", c.name);
            assert!(c.onset > 0, "{} needs a real threshold", c.name);
        }
    }

    #[test]
    fn trust_damps_fear_but_cannot_reach_terror() {
        // Below the onset share, reassurance works: the fear <- trust coupling
        // is negative, so resolved fear falls. A bond that is nothing but fear
        // is fear-dominated at ANY scale — that IS terror under composition
        // gating — while the same fear inside a warm bond stays negotiable.
        let field = AffectField::house();
        let mut afraid = [0i64; CHANNELS];
        afraid[AffectChannel::Fear.index()] = 3_000;
        let bare = field.resolve(&afraid).expect("settles");

        let mut reassured = afraid;
        reassured[AffectChannel::Trust.index()] = 5_000;
        let warm = field.resolve(&reassured).expect("settles");

        let fi = AffectChannel::Fear.index();
        assert!(warm[fi] < bare[fi], "trust must damp fear: {} vs {}", warm[fi], bare[fi]);
        // The coupling itself spreads a fear-only drive into grievance and
        // ignorance, so even the settled "pure fear" bond sits under the
        // calibrated dominance onset — and reassurance moves it further down.
        let share = |f: &[i64; CHANNELS]| f[fi] * Q15 / field_magnitude(f);
        assert!(share(&warm) < share(&bare), "reassurance must lower fear's share of the bond");
        assert_eq!(crossed_weight(&warm), 0, "fear held inside trust stays below the share onset");
    }

    #[test]
    fn a_crossed_state_rides_entropy_and_cannot_be_loved_away() {
        let calm = [0i64; CHANNELS];
        let mut terrified = [0i64; CHANNELS];
        terrified[AffectChannel::Fear.index()] = 25_000;
        terrified[AffectChannel::Trust.index()] = 8_000;

        assert_eq!(crossed_weight(&calm), 0);
        let weight = crossed_weight(&terrified);
        assert!(weight > 0, "fear at 25_000 against trust at 8_000 dominates the bond");

        let t = to_triad(&terrified, 0);
        assert_eq!(t.entropy, weight as i32, "the crossing lands in entropy, not strife");

        // Entropy takes its cut regardless of how much love is present.
        let without = to_triad(&{ let mut f = terrified; f[AffectChannel::Fear.index()] = 0; f }, 0);
        assert!(t.disposition() < without.disposition(), "terror costs disposition even under goodwill");
    }

    #[test]
    fn ignorance_crosses_into_denial_on_the_same_law_as_fear() {
        let denial = CROSSINGS[1];
        assert_eq!(denial.channel, AffectChannel::Ignorance);
        let mut f = [0i64; CHANNELS];
        f[AffectChannel::Ignorance.index()] = 3_000;
        f[AffectChannel::Trust.index()] = 10_000;
        assert!(!denial.crossed(&f), "merely uninformed inside a warm bond is not denial");
        f[AffectChannel::Ignorance.index()] = 12_000;
        f[AffectChannel::Trust.index()] = 3_000;
        assert!(denial.crossed(&f), "an ignorance-dominated bond is a different thing");
        assert_eq!(crossings_in_force(&f), [("denial", denial.amount(&f))]);
    }

    #[test]
    fn the_mapping_reaches_all_three_dispositions() {
        // The property both earlier hand-picked HermeticStats->Triad attempts
        // failed: love and strife must be able to outweigh each other.
        let field = AffectField::house();

        let warm = standing([
            (AffectChannel::Trust, 6_000),
            (AffectChannel::Obligation, 4_000),
            (AffectChannel::Debt, 0),
        ]);
        let sour = standing([
            (AffectChannel::Fear, 5_000),
            (AffectChannel::Grievance, 6_000),
            (AffectChannel::Trust, 0),
        ]);

        let warm_t = triad_of(&warm, &field, 0).expect("settles");
        let sour_t = triad_of(&sour, &field, 0).expect("settles");

        assert_eq!(quantize_disposition(&warm_t), 1, "trust and obligation read as bound");
        assert_eq!(quantize_disposition(&sour_t), -1, "fear and grievance read as torn");

        let empty = AffectMemory::house();
        let empty_t = triad_of(&empty, &field, 0).expect("settles");
        assert_eq!(quantize_disposition(&empty_t), 0, "a clean slate is the fixed point");

        let signs: Vec<i8> = [warm_t, empty_t, sour_t].iter().map(quantize_disposition).collect();
        assert_eq!(signs, [1, 0, -1], "all three signs are reachable");
    }

    #[test]
    fn debt_rides_entropy_and_never_binds() {
        // Entropy takes its cut regardless: an owed debt cannot improve a bond,
        // only tax it. Same reading Triad::disposition already gives entropy.
        let field = AffectField::house();
        let mut owing = AffectMemory::house();
        owing.inject(AffectChannel::Trust, 4_000);
        let before = triad_of(&owing, &field, 0).expect("settles").disposition();
        owing.inject(AffectChannel::Debt, 3_000);
        let after = triad_of(&owing, &field, 0).expect("settles").disposition();
        assert!(after < before, "taking on debt can only cost disposition: {after} vs {before}");
    }

    #[test]
    fn the_rooms_haunt_adds_to_entropy_and_is_bounded() {
        let field = AffectField::house();
        let m = AffectMemory::house();
        let calm = triad_of(&m, &field, 0).expect("settles");
        let haunted = triad_of(&m, &field, 5_000).expect("settles");
        assert_eq!(haunted.entropy - calm.entropy, 5_000);
        let pegged = triad_of(&m, &field, u32::MAX).expect("settles");
        assert_eq!(pegged.entropy, Triad::HAUNT_MAX, "haunt cannot exceed its own ceiling");
    }

    // The claim table and the matrix must be one fact, not two. Every nonzero
    // entry is named; every named entry is nonzero and carries the sign its
    // words claim; no claim sits on a zero and no zero hides an unnamed entry.
    #[test]
    fn the_claim_table_and_the_matrix_are_the_same_fact() {
        let mut named = [[false; CHANNELS]; CHANNELS];
        for c in COUPLING_CLAIMS {
            let (i, j) = (c.of.index(), c.by.index());
            assert!(!named[i][j], "{} <- {} claimed twice", c.of.name(), c.by.name());
            named[i][j] = true;
            let v = HOUSE_COUPLING[i][j];
            assert_ne!(v, 0, "\"{}\" claims a coupling the matrix does not have", c.claim);
            assert_eq!(v > 0, c.raises, "\"{}\" disagrees with the entry's sign ({v})", c.claim);
        }
        for i in 0..CHANNELS {
            assert_eq!(HOUSE_COUPLING[i][i], 0, "a channel must not fold into itself");
            for j in 0..CHANNELS {
                assert_eq!(
                    HOUSE_COUPLING[i][j] != 0,
                    named[i][j],
                    "{} <- {} is in the matrix with no claim behind it",
                    AffectChannel::ALL[i].name(),
                    AffectChannel::ALL[j].name()
                );
            }
        }
    }

    // The entry's sign is not the claim. The claim is about the SETTLED field,
    // and (I-M)^-1 is a different matrix from M — a second-order path can in
    // principle outrun the direct one and reverse it. This asserts it does not.
    #[test]
    fn every_claim_survives_the_coupling() {
        let field = AffectField::house();
        for c in COUPLING_CLAIMS {
            let moved = resolved_response(&field, c.of, c.by).expect("the house field settles");
            assert_ne!(moved, 0, "\"{}\" makes no difference at all once resolved", c.claim);
            assert_eq!(
                moved > 0,
                c.raises,
                "\"{}\" is FALSE through the field: resolved {} moved {moved} when {} rose {PROBE_DELTA}",
                c.claim,
                c.of.name(),
                c.by.name()
            );
        }
    }

    // Every authored entry moves the settled disposition. This is the CHEAP
    // diagnostic over 9 hand-picked probes; the flip counts it reports are a
    // floor, not a measurement — see
    // `every_authored_entry_changes_a_real_reading` for the sampled truth.
    #[test]
    fn every_entry_moves_the_settled_field() {
        let rows = entry_sensitivity();
        assert_eq!(rows.len(), COUPLING_CLAIMS.len());
        for r in &rows {
            assert_ne!(r.authored, 0);
            assert!(r.sign_flips <= r.probes * SWEEP.len() as u32);
            assert!(
                r.disposition_span > 0,
                "{} <- {} moves the settled disposition by nothing at all — it should be zero, not authored",
                r.of.name(),
                r.by.name()
            );
        }
        for r in &rows {
            eprintln!(
                "affect/sens {:<11} <- {:<11} {:>6}  flips {:>2}/{:<2} span {:>7} unsettled {:>2} refused {}",
                r.of.name(),
                r.by.name(),
                r.authored,
                r.sign_flips,
                r.probes * SWEEP.len() as u32,
                r.disposition_span,
                r.unsettled,
                r.refused
            );
        }
    }

    // The distinction the span column exists to make. Every authored entry
    // moves the settled disposition — none is inert IN THE FIELD. The banded
    // three-level quantizer at the end still discards most of that span; the
    // span column is what shows the work it discards.
    #[test]
    fn the_span_column_shows_every_entry_doing_work_in_the_field() {
        let rows = entry_sensitivity();
        for r in &rows {
            assert!(
                r.disposition_span > 0,
                "{} <- {} moves the settled disposition by nothing at all — it should be zero, not authored",
                r.of.name(),
                r.by.name()
            );
        }
        // NOTE: this 9-probe view reports only 3 of 15 entries flipping a sign.
        // That figure is a SAMPLING ARTIFACT and is deliberately not asserted —
        // see `every_authored_entry_changes_a_real_reading`, where proper
        // sampling shows all 15 do.
        eprintln!("affect/sens {} entries move the settled field", rows.len());
    }

    // CORRECTION, 2026-08-25. An earlier pass concluded from 9 hand-picked
    // probes that only 3 of 15 entries survive `quantize_disposition`. That was
    // a sampling artifact. Sampling the drive space properly (`cargo xtask
    // mudlex sim`, 342M resolves) shows ALL 15 change real readings, at
    // 0.51%..3.60% of sparse drive space. This test pins the corrected fact at a
    // budget a unit test can afford, using sign reversal — the sharpest
    // falsifier — so the wrong conclusion cannot be drawn again.
    #[test]
    fn every_authored_entry_changes_a_real_reading() {
        const SAMPLES: u64 = 10_000;
        let house = AffectModel::house();
        for c in COUPLING_CLAIMS {
            let authored = HOUSE_COUPLING[c.of.index()][c.by.index()];
            let flipped = variant_entry(c.of, c.by, -authored).expect("negation stays norm-admissible");
            let d = compare_range(&house, &flipped, DriveRegime::Sparse, 0, SAMPLES);
            assert!(
                d.ta_flips > 0,
                "\"{}\" ({} <- {}) could be written with the opposite sign and no reading in {SAMPLES} \
                 sparse drives would change — that entry would be untestable and should be zero",
                c.claim,
                c.of.name(),
                c.by.name()
            );
        }
    }

    // INVERTED 2026-08-25 from `the_neutral_reading_is_unreachable_...`, which
    // pinned the defect this weld closes. Under the banded direction read the
    // middle register is a solid-angle band, so `Ta = 0` has a live source and
    // the trit axis is ternary in play — the `ahaha` Transformative register
    // fires on near-balanced bonds, not only on a literally empty memory.
    #[test]
    fn neutral_register_is_reachable() {
        let house = AffectModel::house();
        let c = census_range(&house, DriveRegime::Sparse, 0, 20_000);
        assert!(
            c.neutral > 0,
            "no sparse drive read neutral — the middle register lost its live source again"
        );
        assert!(c.bound > 0 && c.torn > 0, "all three registers must stay live");
        assert_eq!(house.reading(&[0i64; CHANNELS]), Some(0), "the empty bond is the fixed point");
        eprintln!(
            "affect/census sparse: bound {} / neutral {} / torn {} permyriad",
            c.permyriad(c.bound),
            c.permyriad(c.neutral),
            c.permyriad(c.torn)
        );
    }

    // The truncation cycle is not a curiosity of the deleted 5-channel draft.
    // Measured on the LIVE house matrix: 0.70% of sparse drives and 0.60% of
    // dense drives never reach an exact integer fixed point. About 1 bond in
    // 140 would report None to a caller that assumed it always settles.
    #[test]
    fn the_live_house_matrix_fails_to_settle_on_a_measurable_slice_of_its_own_space() {
        let house = AffectModel::house();
        let c = census_range(&house, DriveRegime::Sparse, 0, 20_000);
        assert!(
            c.unsettled > 0,
            "no unsettled drive found — if the kernel's truncation cycle was fixed upstream, \
             this test should be retired deliberately, not left passing by accident"
        );
        let rate = c.permyriad(c.unsettled);
        assert!(
            (10..500).contains(&rate),
            "the non-settling rate moved off its measured 0.70%: now {rate} permyriad. \
             A caller of AffectField::resolve must still handle None."
        );
    }

    // The onsets are authored too, and the same question applies: does moving a
    // threshold change anything a player reads, or is 4_000 an unfalsifiable
    // number? A threshold nothing reacts to is not a threshold.
    #[test]
    fn a_crossing_onset_must_be_a_threshold_something_reacts_to() {
        let rows = onset_sensitivity();
        assert_eq!(rows.len(), CROSSINGS.len());
        for r in &rows {
            eprintln!(
                "affect/onset {:<8} {:>6}  flips {:>2}  crossing-changes {:>2}/{:<2} span {:>7}",
                r.name, r.authored, r.sign_flips, r.crossing_changes, r.probes * (ONSET_SWEEP.len() as u32 - 1), r.disposition_span
            );
            assert!(
                r.crossing_changes > 0,
                "{}'s onset can move across the whole sweep without any probe crossing differently — \
                 the threshold is outside the range the field reaches",
                r.name
            );
        }

        // Composition gating inverts the old 97.8% fact: a dense drive lights
        // every channel at comparable size, so fear-DOMINANCE is the exception
        // there, not the default. Terror belongs to sparse, lopsided bonds.
        let house = AffectModel::house();
        let c = census_range(&house, DriveRegime::Dense, 0, 20_000);
        let rate = c.permyriad(c.terror);
        eprintln!("affect/onset terror rate on dense drives: {rate} permyriad");
        assert!(
            rate < 2_000,
            "terror fires on {rate} permyriad of dense drives — dominance should be rare in a dense mix"
        );
    }

    // The whole point of a counter-based draw: a range of sample indices can be
    // split across any number of threads in any order and the census is
    // bit-identical. If this fails the parallel driver is reporting noise.
    #[test]
    fn a_census_is_the_same_however_it_is_split() {
        let house = AffectModel::house();
        for regime in [DriveRegime::Dense, DriveRegime::Sparse] {
            let whole = census_range(&house, regime, 0, 3_000);
            let split = census_range(&house, regime, 0, 701)
                .merge(census_range(&house, regime, 701, 1_999))
                .merge(census_range(&house, regime, 1_999, 3_000));
            assert_eq!(whole, split, "{regime:?}: a census must not depend on how it was cut");
            assert_eq!(whole.samples, 3_000);
            assert_eq!(whole.bound + whole.neutral + whole.torn + whole.unsettled, whole.samples);
        }
    }

    // A drive is a pure function of its index — no carried state, no thread
    // affinity, reproducible across runs and machines.
    #[test]
    fn a_sampled_drive_is_stateless_and_inside_its_envelope() {
        for i in [0u64, 1, 7, 4_242, u64::MAX / 2] {
            for regime in [DriveRegime::Dense, DriveRegime::Sparse] {
                let a = sample_drive(i, regime);
                assert_eq!(a, sample_drive(i, regime), "the same index must draw the same drive");
                assert!(a.iter().all(|v| (0..=DRIVE_CEILING).contains(v)), "{a:?} left the envelope");
            }
        }
        // Sparse means sparse: at most three channels lit, and never all six.
        for i in 0..500u64 {
            let lit = sample_drive(i, DriveRegime::Sparse).iter().filter(|v| **v != 0).count();
            assert!((1..=3).contains(&lit) || lit == 0, "sparse drew {lit} lit channels");
        }
    }

    // Comparing the house model against itself must find exactly nothing. The
    // null result that proves the comparator is not manufacturing divergence.
    #[test]
    fn a_model_does_not_diverge_from_itself() {
        let house = AffectModel::house();
        for regime in [DriveRegime::Dense, DriveRegime::Sparse] {
            let d = compare_range(&house, &house, regime, 0, 2_000);
            assert_eq!(d.ta_flips, 0, "{regime:?}: a model must agree with itself");
            assert_eq!(d.settling_differs, 0);
            assert_eq!(d.disposition_gap, 0);
            assert!(d.both_settled > 0, "{regime:?}: nothing settled, the comparison proved nothing");
        }
    }

    // AffectModel::disposition must be the same fact as to_triad + disposition,
    // or the mass sweep is measuring a second implementation of the game rule.
    #[test]
    fn the_sim_reads_the_same_disposition_the_game_does() {
        let house = AffectModel::house();
        let field = AffectField::house();
        for i in 0..2_000u64 {
            let drive = sample_drive(i, DriveRegime::Sparse);
            match (house.disposition(&drive), field.resolve(&drive)) {
                (Some(d), Some(f)) => {
                    assert_eq!(d, to_triad(&f, 0).disposition() as i64, "drive {drive:?}");
                    assert_eq!(house.reading(&drive), Some(quantize_disposition(&to_triad(&f, 0))));
                }
                (None, None) => {}
                (a, b) => panic!("sim and game disagree on settling: {a:?} vs {}", b.is_some()),
            }
        }
    }

    // ROOT CAUSE (Sean 2026-08-25): `m[i][j] * f[j]` is the affect equivalent of
    // `if humidity > 0.8 { tarnish += 0.05 }` — it asserts propagation instead of
    // deriving it across an interface. A real transfer is driven by a GRADIENT
    // and limited by a CONDUCTANCE and a CAPACITY. A scalar multiply has none of
    // the three, and the consequence is measurable: the field is homogeneous.
    //
    // resolve(k·g) == k·resolve(g), to flooring distance. Doubling every feeling
    // doubles the settled field, forever. Nothing saturates, nothing floors,
    // fear keeps eating trust long after there is no trust left to eat.
    #[test]
    fn the_coupling_is_scale_free_because_it_has_no_capacity_anywhere() {
        let field = AffectField::house();
        for i in 0..400u64 {
            let g = sample_drive(i, DriveRegime::Sparse);
            let Some(base) = field.resolve(&g) else { continue };
            for k in [2i64, 5, 10] {
                let scaled: [i64; CHANNELS] = core::array::from_fn(|c| g[c] * k);
                let Some(big) = field.resolve(&scaled) else { continue };
                for c in 0..CHANNELS {
                    let want = base[c] * k;
                    assert!(
                        (big[c] - want).abs() <= 8 * k,
                        "channel {c}: resolve({k}g) = {} but {k}*resolve(g) = {want}. If this \
                         ever fails, something in the chain finally acquired a capacity.",
                        big[c]
                    );
                }
            }
        }
    }

    // INVERTED 2026-08-25 from `a_fixed_onset_measures_the_scale_of_the_drive_
    // not_the_state_of_the_bond`, which pinned the defect this weld closes.
    // A share onset reads composition, and composition does not move when the
    // drive scales: the terror rate is the same at 1x and 100x. The only scale
    // effect left is FELT_FLOOR, which gates existence, not kind.
    #[test]
    fn crossings_gate_on_share_not_scale() {
        let house = AffectModel::house();
        let terror = CROSSINGS[0];
        let rate_at = |k: i64| -> u64 {
            let (mut crossed, mut settled) = (0u64, 0u64);
            for i in 0..3_000u64 {
                let g = sample_drive(i, DriveRegime::Sparse);
                let scaled: [i64; CHANNELS] = core::array::from_fn(|c| g[c] * k / 100);
                let Some(f) = house.field.resolve(&scaled) else { continue };
                settled += 1;
                if terror.crossed(&f) {
                    crossed += 1;
                }
            }
            if settled == 0 { 0 } else { crossed * PMY / settled }
        };

        let tiny = rate_at(1);
        let authored = rate_at(100);
        let ten = rate_at(1_000);
        let thousand = rate_at(100_000);
        eprintln!("affect/scale terror rate: 1/100x {tiny}, 1x {authored}, 10x {ten}, 1000x {thousand} permyriad");
        assert!(authored > 0, "some sparse bonds are fear-dominated");
        // Above the felt floor, composition does not move with scale.
        assert!(
            (ten as i64 - thousand as i64).abs() <= 100,
            "composition gating must not move with drive scale: {ten} vs {thousand} permyriad"
        );
        // Below it, bonds stop existing — the floor only ever REMOVES crossings.
        assert!(authored <= ten, "the felt floor can only remove crossings: {authored} vs {ten}");
        assert!(tiny <= authored, "at 1/100 scale almost nothing is felt at all: {tiny}");
    }

    #[test]
    fn the_whole_path_reaches_a_cry() {
        use crate::ironroot::lexicon::Cry;
        use crate::ironroot::trit_grammar::TritReading;

        let field = AffectField::house();
        let sour = standing([
            (AffectChannel::Fear, 5_000),
            (AffectChannel::Grievance, 6_000),
            (AffectChannel::Trust, 0),
        ]);
        let triad = triad_of(&sour, &field, 0).expect("settles");
        let reading = TritReading { ta: quantize_disposition(&triad), tn: 0, tv: 0 };
        assert_eq!(reading.ta, Cry::Evocative.trit(), "a torn bond speaks in ah");
        assert!(!reading.bearing().is_origin(), "a felt bond has a bearing");
    }
}
