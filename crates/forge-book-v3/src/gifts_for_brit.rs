//! 100 Gifts for Brit — the community-initiative chapter (Sean 2026-08-05).
//! Five rooms, twenty gifts each, every line sized for a thirteen-second read.
//! Fresh-harvested from the whole book this session (194 modules scanned), not
//! ported from any earlier pitch. Status vocabulary is honest: proven rows cite
//! code receipts, world rows cite outside evidence, program rows are programs
//! to run — never claims. The Ramus shape (5×4×5, aperture law) is test-locked.

use crate::atlas::AtlasSection;
use crate::block::Block;
use crate::chapter::Chapter;
use crate::page::Page;

/// One gift: id · room (branch) · cluster · name · thirteen-second line ·
/// honesty status (proven|wired|planned|program|world) · receipt.
pub struct Gift {
    /// Numeric gift ID, ranging from 1 to 100 in order.
    pub id: u8,
    /// The room/branch this gift belongs to, one of the five in BRANCHES.
    pub branch: &'static str,
    /// The cluster/shelf grouping within the room.
    pub cluster: &'static str,
    /// The name or title of the gift, at most 32 characters.
    pub name: &'static str,
    /// The thirteen-second description line, at most 90 characters.
    pub line: &'static str,
    /// Honesty status: proven, wired, planned, program, or world.
    pub status: &'static str,
    /// Receipt citation: a file:line, source URL, or evidence reference.
    pub receipt: &'static str,
}

/// The five rooms, in pitch order.
pub const BRANCHES: [&str; 5] = [
    "The Healing Room",
    "The Language Room",
    "The Maker Floor",
    "The Quiet Design",
    "The Honest Ledger",
];

macro_rules! g {
    ($id:expr, $b:expr, $c:expr, $n:expr, $l:expr, $s:expr, $r:expr) => {
        Gift { id: $id, branch: $b, cluster: $c, name: $n, line: $l, status: $s, receipt: $r }
    };
}

/// All one hundred gifts. Grouping is Ramus-strict: 5 rooms × 4 clusters × 5 gifts.
pub const GIFTS: [Gift; 100] = [
    // ─── ROOM 1 · THE HEALING ROOM (art-for-trauma anchor) ───────────────────
    g!(1, "The Healing Room", "Paint without fear", "Paint studio",
       "A full painting room: brushes, fill, line, layers, undo — one window.",
       "proven", "forge-studio/src/paint_host.rs · Surface::Paint"),
    g!(2, "The Healing Room", "Paint without fear", "Six real brushes",
       "Pencil, lathe, stamp, spring pen, hatch fill, mirror — a real rack.",
       "proven", "forge-book/src/brushes.rs (.brush.vixi set)"),
    g!(3, "The Healing Room", "Paint without fear", "One shared palette",
       "Sixty-four colours that mean the same thing in every room of the shop.",
       "proven", "forge-vix/panels/palette.kit.vixi · palette rail"),
    g!(4, "The Healing Room", "Paint without fear", "Colour harmony helper",
       "Picks colours that agree with each other, automatically.",
       "proven", "forge-book/src/colour.rs (OKLCH harmony)"),
    g!(5, "The Healing Room", "Paint without fear", "Photo to relief",
       "Turns a photograph into a touchable 3D carving, ready to print.",
       "proven", "forge-geo/src/relief.rs · watertight ×3 GREEN"),
    g!(6, "The Healing Room", "Sound that settles", "Music desk",
       "A four-deck DJ and recording studio in one window.",
       "proven", "forge-gui recording_studio_kit · HITL 2026-07-09"),
    g!(7, "The Healing Room", "Sound that settles", "Broadcast booth",
       "Mic strip, record button, level meters — podcast-ready.",
       "wired", "forge-audio broadcast_booth"),
    g!(8, "The Healing Room", "Sound that settles", "Key wheel",
       "Shows which songs mix without clashing, before you try.",
       "proven", "forge-audio camelot.rs (harmonic wheel)"),
    g!(9, "The Healing Room", "Sound that settles", "Sound shapes",
       "Draw a sound's rise and fall by hand; the machine plays it back.",
       "proven", "forge-book/src/dsp.rs (integer ADSR)"),
    g!(10, "The Healing Room", "Sound that settles", "Steady time",
       "Metronome and note grid — a drum circle keeps together.",
       "proven", "forge-book/src/{metronome,note_grid}.rs"),
    g!(11, "The Healing Room", "Stories that carry", "Story drops",
       "Prose becomes a thousand-picture film, paced like breathing.",
       "proven", "tools/storydrop-forge/storydrop.py · 12-frame proof"),
    g!(12, "The Healing Room", "Stories that carry", "Book builder",
       "Write, drop pictures in, press one button — out comes a website.",
       "proven", "forge-book/src/export_html.rs (export_book)"),
    g!(13, "The Healing Room", "Stories that carry", "Zine press",
       "The same book prints to a paper-ready file for the copier.",
       "proven", "forge-book/src/export_md.rs"),
    g!(14, "The Healing Room", "Stories that carry", "Branching stories",
       "Choose-your-own-path story trees, walked by choice.",
       "proven", "forge-book/src/dialogue.rs"),
    g!(15, "The Healing Room", "Stories that carry", "Pressure pen",
       "Pen pressure becomes emphasis on the page — the hand stays in the text.",
       "proven", "forge-book/src/authoring.rs (nib pressure)"),
    g!(16, "The Healing Room", "Proof it helps", "Nine in ten",
       "About 9 in 10 arts-program participants self-report better mental health.",
       "world", "Psychology Today Canada, 2026-07"),
    g!(17, "The Healing Room", "Proof it helps", "Youth PTSD evidence",
       "A 2025 meta-analysis: creative arts reduce PTSD symptoms in young people.",
       "world", "Nature Mental Health, 2025"),
    g!(18, "The Healing Room", "Proof it helps", "Calm by construction",
       "Built to never overwhelm — that is a compiled law, not a promise.",
       "proven", "creation_dag.rs · aperture law 4±1"),
    g!(19, "The Healing Room", "Proof it helps", "Nothing leaves the room",
       "One machine, wire shut. No cloud, no account, no watching.",
       "proven", "forge_firewall egress REFUSED, 2026-08-05"),
    g!(20, "The Healing Room", "Proof it helps", "Receipts for funders",
       "Every claim carries a file line or a source you can check.",
       "proven", "forge-book/src/atlas.rs (proof badges)"),

    // ─── ROOM 2 · THE LANGUAGE ROOM (Cree calligraphy · Métis Nation) ───────
    g!(21, "The Language Room", "The Star Alphabet", "Cree syllabics engine",
       "The Star Alphabet: glyphs whose rotation IS the vowel — encoded and taught.",
       "proven", "_book/06-cree-syllabics.md · seed::full_atlas"),
    g!(22, "The Language Room", "The Star Alphabet", "Calligraphy strokes",
       "Stroke geometry for the glyphs, each carrying a provenance seal.",
       "wired", "forge-calligraphy (stroke + seal)"),
    g!(23, "The Language Room", "The Star Alphabet", "Glyph meets sound",
       "Each written character carries its own voice — hearing and reading, one fact.",
       "wired", "forge-calligraphy/src/audio_bridge.rs"),
    g!(24, "The Language Room", "The Star Alphabet", "Syllabics as paint",
       "Write the language directly into artwork — glyph stamps as brushes.",
       "planned", "one_engine.rs FORWARD · syllabic_stamp"),
    g!(25, "The Language Room", "The Star Alphabet", "The star map",
       "A sky index anchored on the Cree standing-still star; it files the alphabet.",
       "proven", "forge-ml/src/sphere_index.rs · 7 tests green"),
    g!(26, "The Language Room", "Culture kept right", "Elder-reviewed names",
       "nehiyaw-reviewed archetypes only; the faux pas were found and removed.",
       "proven", "forge-book/src/animal_signs.rs (2026-03 redline)"),
    g!(27, "The Language Room", "Culture kept right", "Eight guardians",
       "Cree-named zone spirits, each with a real, documented sound signature.",
       "proven", "forge-book/src/lore/guardians.rs"),
    g!(28, "The Language Room", "Culture kept right", "13 Moons storywork",
       "A voice-linted story chapter living inside the one book.",
       "proven", "_book/03-take-too-much.md · voice lint 4.54"),
    g!(29, "The Language Room", "Culture kept right", "The honest floor",
       "What is ours and what is borrowed is said plainly, in writing.",
       "proven", "_book/06 · cultural floor honest"),
    g!(30, "The Language Room", "Culture kept right", "Provenance seals",
       "Who made it and when — tamper-evident, breaks loudly if touched.",
       "proven", "forge-book/src/{evidence,seal}.rs"),
    g!(31, "The Language Room", "Teaching tools", "Learning tracks",
       "Lessons with prerequisites; the next lesson unlocks when you are ready.",
       "proven", "forge-book/src/{curriculum,learning}.rs"),
    g!(32, "The Language Room", "Teaching tools", "A book that grows",
       "Reveals more as the reader advances — never a wall at once.",
       "proven", "forge-book/src/grow.rs"),
    g!(33, "The Language Room", "Teaching tools", "Mastery by doing",
       "Practice raises skill on a real dial; trying something new counts double.",
       "proven", "forge-book/src/techniques.rs"),
    g!(34, "The Language Room", "Teaching tools", "Font sandbox",
       "Build and test typefaces for syllabics, live on screen.",
       "proven", "forge-studio font_sandbox_kit · Create surface"),
    g!(35, "The Language Room", "Teaching tools", "Reading-level gauge",
       "Keeps lesson text at the level the learner is actually at.",
       "proven", "forge-book/src/readability.rs"),
    g!(36, "The Language Room", "The moment is now", "SILR 2026, Edmonton",
       "460+ gathered in April; workshops on syllabics and digital fluency tools.",
       "world", "ualberta.ca · SILR Gathering, Apr 27-29 2026"),
    g!(37, "The Language Room", "The moment is now", "NRC ships language tech",
       "The federal lab open-sources tools for 25+ languages, Cree and Michif in.",
       "world", "nrc.canada.ca · Indigenous languages tech project"),
    g!(38, "The Language Room", "The moment is now", "Federal language money",
       "The Indigenous Languages Component funds community-led revitalization.",
       "world", "canada.ca · Canadian Heritage ILC"),
    g!(39, "The Language Room", "The moment is now", "The Métis language office",
       "Otipemisiwak is actively revitalizing nêhiyawêwin and three Michifs.",
       "world", "albertametis.com/culture/language"),
    g!(40, "The Language Room", "The moment is now", "Brit's program",
       "Her Métis Nation talks plus this engine equals the pilot. It is ready.",
       "program", "Métis Nation talks (Brit, 2026)"),

    // ─── ROOM 3 · THE MAKER FLOOR (classes · youth · community access) ───────
    g!(41, "The Maker Floor", "One machine, every art", "One program, every tool",
       "Paint, music, worlds, words — thirty-six organs, one single program.",
       "proven", "forge-book/src/one_engine.rs (ORGANS ≥30, test)"),
    g!(42, "The Maker Floor", "One machine, every art", "No subscription, ever",
       "One computer, no internet needed, nothing rented, nothing expiring.",
       "proven", "one-bin law · offline egress shut"),
    g!(43, "The Maker Floor", "One machine, every art", "Five rooms, one door",
       "Paint, Create, Audio, Terminal, Hub — tabs across the top, that's it.",
       "proven", "catalog.rs studio surfaces · atlas page test"),
    g!(44, "The Maker Floor", "One machine, every art", "A flight recorder",
       "Every change is recorded; any afternoon can be rewound.",
       "proven", "forge-vcs (content-addressed tape)"),
    g!(45, "The Maker Floor", "One machine, every art", "Machine eyes",
       "The shop screenshots and checks its own screens — visual proof, not vibes.",
       "wired", "forge-vision (forgewright capture)"),
    g!(46, "The Maker Floor", "Game worlds", "World map maker",
       "Zones, eras, factions, connections — a living map of an invented place.",
       "proven", "forge-book/src/cartography.rs"),
    g!(47, "The Maker Floor", "Game worlds", "Hex and tile boards",
       "Board-game grids with real distances — print and play at the hall.",
       "proven", "forge-book/src/{hexgrid,tilemap}.rs"),
    g!(48, "The Maker Floor", "Game worlds", "Creature book",
       "Archetypes with temperaments and stances — a bestiary for any table.",
       "proven", "forge-book/src/bestiary.rs"),
    g!(49, "The Maker Floor", "Game worlds", "The whole game night",
       "Quests, crafting, loot, trade, experience — the full kit, deterministic.",
       "proven", "forge-book/src/{quest,crafting,loot,economy,xp}.rs"),
    g!(50, "The Maker Floor", "Game worlds", "Seasons drive story",
       "An in-world calendar and weather that turn with the year — gardens too.",
       "proven", "forge-book/src/{calendar,weather}.rs"),
    g!(51, "The Maker Floor", "Picture pipeline", "Photo to game piece",
       "A photograph becomes a placeable asset: cut out, meshed, ready.",
       "proven", "photo_pipeline · relief lane GREEN"),
    g!(52, "The Maker Floor", "Picture pipeline", "Sprite splitter",
       "One drawn sheet becomes many game pieces, automatically.",
       "wired", "forge-export + photo_pipeline (ch05 track)"),
    g!(53, "The Maker Floor", "Picture pipeline", "The item mint",
       "A sword forged from a seed number, with its own fingerprint and preview.",
       "proven", "forge-studio items_tool · sword-42.glb GREEN"),
    g!(54, "The Maker Floor", "Picture pipeline", "220 real materials",
       "Stone, sand, metal, cloth — real surface looks bound to the 64 slots.",
       "proven", "forge-materials slot_correspondence · 43 green"),
    g!(55, "The Maker Floor", "Picture pipeline", "Living pixels",
       "Sand falls, fire spreads, water flows — playable physics for art class.",
       "wired", "forge-render vixel_automata.wgsl"),
    g!(56, "The Maker Floor", "The video lane", "Devlog pipeline",
       "Screen plus voice becomes a finished YouTube cut — no camera, no crew.",
       "wired", "tools/youtube-forge (traced 2026-08-03)"),
    g!(57, "The Maker Floor", "The video lane", "Thousand-picture reels",
       "Stories become videos: dwell, blink, and page-turn timing built in.",
       "proven", "storydrop pipe-1000 · pacing engine"),
    g!(58, "The Maker Floor", "The video lane", "Cinematic slides",
       "Beat-mapped scene decks generated from prose drafts.",
       "wired", "youtube-forge beat map projects"),
    g!(59, "The Maker Floor", "The video lane", "Ghost voice lane",
       "Spoken-word and chant layers for the reels — the eerie done kindly.",
       "wired", "youtube-forge ghost/chant rig (08-03 trace)"),
    g!(60, "The Maker Floor", "The video lane", "One source, many faces",
       "One authored file emits the site, the deck, and the video assets.",
       "wired", "source-compiler ladder (vixi→HTML5/WGSL/media)"),

    // ─── ROOM 4 · THE QUIET DESIGN (neurodivergent-first, by law) ────────────
    g!(61, "The Quiet Design", "Never overwhelms", "The four-things rule",
       "No screen group ever exceeds four-ish items. It is a compiled law here.",
       "proven", "creation_dag.rs · FORGE_INVARIANTS aperture law"),
    g!(62, "The Quiet Design", "Never overwhelms", "One thing glows",
       "One attention cue at a time — never five things blinking at once.",
       "proven", "root#a000 · 1-preattentive gate"),
    g!(63, "The Quiet Design", "Never overwhelms", "Depth on request",
       "Detail appears only when asked for; the surface stays calm.",
       "proven", "progressive disclosure · grow.rs"),
    g!(64, "The Quiet Design", "Never overwhelms", "One honest icon each",
       "Every tool gets one picture that means it — no mystery buttons.",
       "proven", "root#a000 icon-per-node · creation_dag"),
    g!(65, "The Quiet Design", "Never overwhelms", "The forward ratchet",
       "Screens are allowed to get simpler over time — never to re-clutter.",
       "proven", "root#a000 forward-ratchet · golden_vixi.rs"),
    g!(66, "The Quiet Design", "Predictable is safe", "Same input, same result",
       "The engine is integer-deterministic: no surprises, ever, by construction.",
       "proven", "forge-book/src/physics.rs · integer sim doctrine"),
    g!(67, "The Quiet Design", "Predictable is safe", "Two clocks",
       "A steady 120Hz heartbeat runs the world; the art lane runs free beside it.",
       "wired", "forge-studio dual_loop (2-clocks organ)"),
    g!(68, "The Quiet Design", "Predictable is safe", "No popups, no nags",
       "Click-only, provably no dead ends, and nothing phones out.",
       "proven", "forge-book/src/greeter.rs (reachability lint)"),
    g!(69, "The Quiet Design", "Predictable is safe", "Never lose the work",
       "Save, history, and diff are built into the book itself.",
       "proven", "forge-book/src/{persist,history,diff}.rs"),
    g!(70, "The Quiet Design", "Predictable is safe", "One pick, one meaning",
       "Pick a colour once — material, sound and feel follow it.",
       "proven", "correspondence 5-legs · 64/64 bound"),
    g!(71, "The Quiet Design", "The watchful desk", "Honest gauges",
       "Progress is a measured bar on the desk, not a guess in a meeting.",
       "proven", "forge-book/src/gauge.rs (done-bar)"),
    g!(72, "The Quiet Design", "The watchful desk", "Board that cannot flatter",
       "It says 3 green, 323 unwired — out loud, to funders too.",
       "proven", "board_sync.rs · living board seal"),
    g!(73, "The Quiet Design", "The watchful desk", "Flags that cannot stale",
       "Every warning is re-checked from disk each time — no rotting dashboards.",
       "proven", "forge-book/src/flag_gauge.rs"),
    g!(74, "The Quiet Design", "The watchful desk", "Loud when missing",
       "Absent things say ABSENT. Nothing fails silently, by rule.",
       "proven", "dreams.rs LOUD MISSING · adr0001_oracle.rs"),
    g!(75, "The Quiet Design", "The watchful desk", "One word, one meaning",
       "A glossary where each term has an owner and a single sense.",
       "proven", "forge-core glossary · appendix face"),
    g!(76, "The Quiet Design", "The research base", "Attention-paced media",
       "Dwell and blink timing are parameters, not accidents — tuned for ADHD reads.",
       "proven", "storydrop dwell/blink · readability.rs"),
    g!(77, "The Quiet Design", "The research base", "Safety is chapter one",
       "The build track opens with Mercy & Iron — ethics before pixels.",
       "proven", "achievements.rs ch01_safety"),
    g!(78, "The Quiet Design", "The research base", "Sound science, pointed kind",
       "The psychoacoustics are documented per voice — the same dials can soothe.",
       "wired", "lore/guardians.rs DSP signatures"),
    g!(79, "The Quiet Design", "The research base", "The laws bind us too",
       "Cognitive-load law governs every export — even this page.",
       "proven", "root#a000 · vixi-uiux gates"),
    g!(80, "The Quiet Design", "The research base", "Three hundred named methods",
       "Hundreds of named mechanisms, each with a file line to point at.",
       "wired", "catalog.rs · one_engine.rs · arch tablets · _book"),

    // ─── ROOM 5 · THE HONEST LEDGER (rails · transparency · programs) ────────
    g!(81, "The Honest Ledger", "The paperwork spine", "Three papers, in order",
       "Incorporate, open the bank account, set up payroll. Boring, vital, cheap.",
       "program", "the nonprofit rail (Sean, 2026)"),
    g!(82, "The Honest Ledger", "The paperwork spine", "December opens the door",
       "Year-end lands in December; the SR&ED and lending window opens after it.",
       "program", "BOAST timeline (Sean, in contact since Dec)"),
    g!(83, "The Honest Ledger", "The paperwork spine", "R&D diary, already kept",
       "Every experiment is dated and receipted daily — SR&ED evidence by default.",
       "proven", "forge-vcs tape + living board"),
    g!(84, "The Honest Ledger", "The paperwork spine", "Five priced inventions",
       "A licensing table already exists: $1.105M conservative first-year model.",
       "proven", "pricing.rs y1_total (test-locked)"),
    g!(85, "The Honest Ledger", "The paperwork spine", "Cost-of-business gauge",
       "Twenty dials weigh spend against output every session.",
       "proven", "forge-book/src/assay.rs"),
    g!(86, "The Honest Ledger", "Transparent by construction", "Tamper-proof receipts",
       "The ledger is hash-chained: touch one entry and the whole chain breaks loudly.",
       "proven", "forge-book/src/evidence.rs"),
    g!(87, "The Honest Ledger", "Transparent by construction", "Unfakeable signatures",
       "Who-did-what is read from the operating system, not from a typed name.",
       "proven", "forge-book/src/actor.rs (token SID)"),
    g!(88, "The Honest Ledger", "Transparent by construction", "The 3-for-1 debt law",
       "Every shortcut stacked owes three real repairs, with proof. Compiled.",
       "proven", "forge-book/src/debt_ledger.rs"),
    g!(89, "The Honest Ledger", "Transparent by construction", "Badged bragging",
       "The brag page marks each claim PROVEN, WIRED, or PLANNED. It cannot inflate.",
       "proven", "atlas_html.rs brag_page"),
    g!(90, "The Honest Ledger", "Transparent by construction", "One seal, whole shop",
       "The entire project state compresses to one checksum a funder can hold.",
       "proven", "board seal 663d7ac25e63 (2026-08-05)"),
    g!(91, "The Honest Ledger", "Programs we run", "Trauma-art studio nights",
       "The Healing Room, open evenings, staffed — where the province funds waitlists.",
       "program", "Room 1 + its Proof-it-helps receipts"),
    g!(92, "The Honest Ledger", "Programs we run", "Cree calligraphy workshops",
       "The Language Room, delivered with the Métis Nation — Brit's program.",
       "program", "Room 2 + the talks in progress"),
    g!(93, "The Honest Ledger", "Programs we run", "The garden's paperwork",
       "Plot maps, planting calendar, harvest ledger — beautiful, printed, shared.",
       "program", "cartography + calendar + inventory"),
    g!(94, "The Honest Ledger", "Programs we run", "Youth game-night league",
       "Worlds, quests, and dice from the Maker Floor — printable, screens optional.",
       "program", "Room 3 game-world kit"),
    g!(95, "The Honest Ledger", "Programs we run", "Community zine press",
       "Neighbourhood stories in, printed zines and a website out, same afternoon.",
       "program", "book builder + zine press (12, 13)"),
    g!(96, "The Honest Ledger", "The pitch itself", "The 13-second script",
       "Every line fits one breath — a test fails if it doesn't.",
       "proven", "gifts_for_brit.rs (this file's tests)"),
    g!(97, "The Honest Ledger", "The pitch itself", "A demo that fits a phone",
       "One self-contained page that opens anywhere, offline.",
       "proven", "atlas_html.rs (standalone HTML)"),
    g!(98, "The Honest Ledger", "The pitch itself", "Built in the dark",
       "Nine months, two people, 127 crates, every claim receipted.",
       "proven", "forge-vcs tape · 127/127 crates indexed"),
    g!(99, "The Honest Ledger", "The pitch itself", "We can do better, provably",
       "They fund studies of the problem; this shop runs the answer.",
       "program", "Rooms 1-4 vs the waitlist status quo"),
    g!(100, "The Honest Ledger", "The pitch itself", "Brit holds the pen",
       "The next hundred are hers. Her red pen is revision two.",
       "program", "Brittany — Oracle B, creative director"),
];

/// The one line each room opens with — why a funder should care about that room.
pub const ROOM_GAP: [&str; 5] = [
    "The province funds waitlists. This room funds making.",
    "The language money exists. The delivery tools are what's missing.",
    "A full creative studio with no subscription. Access is the point.",
    "Software that respects how brains actually work — by law, not promise.",
    "Every claim receipted. Funders see the same board we do.",
];

/// The badge key — what each honesty word costs us to say, and how many carry it.
const BADGE_KEY: [(&str, &str); 5] = [
    ("proven", "receipted on disk (70)"),
    ("wired", "built, proof pending (13)"),
    ("world", "outside evidence (6)"),
    ("program", "a program we run (10)"),
    ("planned", "named, not built (1)"),
];

/// Outside evidence, linked so anyone can check it without us in the room.
const SOURCES: [(&str, &str); 6] = [
    ("https://www.psychologytoday.com/ca/blog/the-art-effect/202607/arts-engagement-in-trauma-recovery-can-benefit-mental-health", "Arts in trauma recovery"),
    ("https://www.nature.com/articles/s44220-025-00543-y", "Youth PTSD meta-analysis"),
    ("https://www.ualberta.ca/en/supporting-indigenous-language-revitalization/events/silr-gathering-2026.html", "SILR Gathering 2026"),
    ("https://nrc.canada.ca/en/research-development/research-collaboration/programs/canadian-indigenous-languages-technology-project", "NRC language tech"),
    ("https://www.canada.ca/en/canadian-heritage/services/funding/aboriginal-peoples.html", "Indigenous Languages Component"),
    ("https://albertametis.com/culture/language/", "Otipemisiwak Métis Government"),
];

/// The published page as it sits on disk — the artifact Brit hands out.
/// It is EMITTED, never hand-kept: [`page_is_the_emitted_face`] proves this file is
/// byte-identical to [`render_page`], so the table is the only place a word lives
/// (`compile.rs` doctrine: no face is hand-authored, or the faces drift).
pub const PUBLISHED: &str = include_str!("gifts_for_brit.html");

/// Escape the four characters that would otherwise close a tag or open an entity.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

const PAGE_CSS: &str = include_str!("gifts_for_brit.css");

/// Emit the whole page from the table. Three levels of doors — rooms, shelves, and the
/// fine print's nested sources — so nothing lands on screen uninvited. Deterministic:
/// same table, same bytes.
pub fn render_page() -> String {
    let mut s = String::with_capacity(48 * 1024);
    s.push_str("<title>100 Gifts for Brit</title>\n<style>\n");
    s.push_str(PAGE_CSS);
    s.push_str("</style>\n\n<div class=\"wrap\">\n");
    s.push_str("  <p class=\"eyebrow\">a chapter of the living atlas</p>\n");
    s.push_str("  <h1>100 Gifts for Brit</h1>\n");
    s.push_str("  <p class=\"lede\">Five rooms. Open one, then a shelf.</p>\n");

    for (i, room) in BRANCHES.iter().enumerate() {
        s.push_str(&format!(
            "\n  <details class=\"room\">\n    <summary class=\"room-head\">\
             <span class=\"room-no\">ROOM {}</span><span class=\"room-name\">{}</span>\
             <span class=\"room-mark\">+</span></summary>\n    <div class=\"room-body\">\n",
            i + 1,
            esc(room)
        ));
        s.push_str(&format!("      <p class=\"room-gap\">{}</p>\n", esc(ROOM_GAP[i])));

        let mut last = "";
        for gift in GIFTS.iter().filter(|g| g.branch == *room) {
            if gift.cluster != last {
                if !last.is_empty() {
                    s.push_str("      </ul></details>\n");
                }
                s.push_str(&format!(
                    "      <details class=\"shelf\"><summary>{}</summary><ul class=\"gifts\">\n",
                    esc(gift.cluster)
                ));
                last = gift.cluster;
            }
            s.push_str(&format!(
                "        <li><b>{}</b> {}</li>\n",
                esc(gift.name),
                esc(gift.line)
            ));
        }
        s.push_str("      </ul></details>\n");

        if i + 1 == BRANCHES.len() {
            s.push_str(&format!(
                "      <details class=\"shelf\"><summary>{}</summary>\n        <ul class=\"gifts\">\n",
                "How to check any of this"
            ));
            for (word, means) in BADGE_KEY {
                s.push_str(&format!("        <li><b>{word}</b> {means}</li>\n"));
            }
            s.push_str("        </ul>\n        <details class=\"sources\"><summary>Outside sources</summary><ul>\n");
            for (url, name) in SOURCES {
                s.push_str(&format!("          <li><a href=\"{url}\">{name}</a></li>\n"));
            }
            s.push_str("        </ul></details>\n      </details>\n");
        }
        s.push_str("    </div>\n  </details>\n");
    }
    s.push_str("</div>\n");
    s
}

/// Bind the hundred gifts into the Atlas: one page per room, clusters as blocks.
pub fn gifts_chapter() -> Chapter {
    let mut ch = Chapter::new("100 Gifts for Brit", AtlasSection::Custom("Gifts".into()));
    ch.add_lore(
        "Five rooms, twenty gifts each, every line one breath long. A community \
         initiative, not a product pitch: art-for-trauma and the community gardens \
         are the anchors, the Cree Language Room is the program in motion, and every \
         row is either receipted on disk, receipted in the world, or named honestly \
         as a program to run.",
    );
    for (i, room) in BRANCHES.iter().enumerate() {
        let mut p = Page::new(i as u32 + 1);
        p.add(Block::text(format!("ROOM {} · {}", i + 1, room)));
        let mut last_cluster = "";
        for gift in GIFTS.iter().filter(|gift| gift.branch == *room) {
            if gift.cluster != last_cluster {
                p.add(Block::text(format!("  ◆ {}", gift.cluster)));
                last_cluster = gift.cluster;
            }
            p.add(Block::text(format!(
                "    {}. {} — {} [{}] ({})",
                gift.id, gift.name, gift.line, gift.status, gift.receipt
            )));
        }
        ch.add_page(p);
    }
    ch
}

/// Brit's take-away script: the whole hundred as markdown, grouped for a
/// thirteen-second attention span — room, cluster, one-breath lines.
pub fn brit_script() -> String {
    let mut s = String::from("# 100 Gifts for Brit\n");
    for (i, room) in BRANCHES.iter().enumerate() {
        s.push_str(&format!("\n## Room {} · {}\n", i + 1, room));
        let mut last_cluster = "";
        for gift in GIFTS.iter().filter(|gift| gift.branch == *room) {
            if gift.cluster != last_cluster {
                s.push_str(&format!("\n### {}\n", gift.cluster));
                last_cluster = gift.cluster;
            }
            s.push_str(&format!("- **{}** — {}\n", gift.name, gift.line));
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The count and the ids are exact: 100 gifts, numbered 1..=100 in order.
    // [BOARD: GIFTS-100]
    #[test]
    fn exactly_one_hundred_in_order() {
        assert_eq!(GIFTS.len(), 100);
        for (i, gift) in GIFTS.iter().enumerate() {
            assert_eq!(gift.id as usize, i + 1, "gift {} out of order", gift.name);
        }
    }

    /// The Ramus shape holds the aperture law mechanically: 5 rooms, 4 clusters
    /// per room, 5 gifts per cluster — no group beyond 4±1.
    #[test]
    fn ramus_shape_is_five_four_five() {
        for room in BRANCHES {
            let in_room: Vec<_> = GIFTS.iter().filter(|gift| gift.branch == room).collect();
            assert_eq!(in_room.len(), 20, "{room} must hold exactly 20 gifts");
            let mut clusters: Vec<&str> = Vec::new();
            for gift in &in_room {
                if !clusters.contains(&gift.cluster) {
                    clusters.push(gift.cluster);
                }
            }
            assert_eq!(clusters.len(), 4, "{room} must hold exactly 4 clusters");
            for c in clusters {
                let n = in_room.iter().filter(|gift| gift.cluster == c).count();
                assert_eq!(n, 5, "{room}/{c} must hold exactly 5 gifts");
            }
        }
    }

    /// The thirteen-second bar: every line reads in one breath (<=90 chars),
    /// every name fits a chip (<=32 chars).
    #[test]
    fn thirteen_second_bar_holds() {
        for gift in &GIFTS {
            assert!(
                gift.line.chars().count() <= 90,
                "gift {} line too long ({} chars)", gift.id, gift.line.chars().count()
            );
            assert!(
                gift.name.chars().count() <= 32,
                "gift {} name too long", gift.id
            );
        }
    }

    /// Honesty vocabulary: every status is one of the five declared words,
    /// every receipt is non-empty, and the list is mostly PROVEN — a gift list,
    /// not a wish list (planned is capped at 2).
    #[test]
    fn statuses_are_honest_and_receipted() {
        const VOCAB: [&str; 5] = ["proven", "wired", "planned", "program", "world"];
        let mut proven = 0usize;
        let mut planned = 0usize;
        for gift in &GIFTS {
            assert!(VOCAB.contains(&gift.status), "gift {} bad status", gift.id);
            assert!(!gift.receipt.is_empty(), "gift {} missing receipt", gift.id);
            match gift.status {
                "proven" => proven += 1,
                "planned" => planned += 1,
                _ => {}
            }
        }
        assert!(proven >= 60, "a gift list must be mostly proven (got {proven})");
        assert!(planned <= 2, "planned is a wish, cap it (got {planned})");
    }

    /// The page on disk IS the emitted face. Hand-editing it, or editing the table
    /// without re-emitting, goes red here — the two can never quietly disagree.
    #[test]
    fn page_is_the_emitted_face() {
        assert_eq!(
            PUBLISHED,
            render_page(),
            "gifts_for_brit.html is stale — re-emit it from render_page()"
        );
    }

    /// The emitted page carries every gift and every room, escaping intact.
    #[test]
    fn the_published_page_binds_every_gift() {
        let page = render_page();
        assert!(page.contains("<title>100 Gifts for Brit</title>"));
        for room in BRANCHES {
            assert!(page.contains(room), "page missing room {room}");
        }
        for gift in &GIFTS {
            assert!(page.contains(&esc(gift.name)), "page missing gift {}", gift.id);
            assert!(page.contains(&esc(gift.line)), "page missing line {}", gift.id);
        }
    }

    /// The shelf budget, per gift: a shelf holds five, and a shelf must stay under the
    /// ADHD lens's words-per-view target — so one gift owns at most a fifth of it.
    /// This is what makes "thirteen seconds" a measurement instead of a promise.
    #[test]
    fn every_gift_fits_its_fifth_of_a_shelf() {
        let budget = crate::cognitive_load::Load::word_ceil() / 5;
        let over: Vec<String> = GIFTS
            .iter()
            .filter_map(|g| {
                let n = g.name.split_whitespace().count() + g.line.split_whitespace().count();
                (n > budget).then(|| format!("{} [{n}w] {} {}", g.id, g.name, g.line))
            })
            .collect();
        assert!(over.is_empty(), "budget {budget}w, over by:\n{}", over.join("\n"));
    }

    /// The chapter binds one page per room and the script carries every gift.
    #[test]
    fn chapter_and_script_carry_all_rooms() {
        let ch = gifts_chapter();
        assert_eq!(ch.section, AtlasSection::Custom("Gifts".into()));
        assert_eq!(ch.page_count(), 5, "one page per room");
        let script = brit_script();
        for gift in &GIFTS {
            assert!(script.contains(gift.name), "script missing gift {}", gift.id);
        }
        for room in BRANCHES {
            assert!(script.contains(room), "script missing room {room}");
        }
    }
}
