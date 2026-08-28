//! Birth-rite interview state machine (W4) — headless, glass-free: the cart
//! authors the questions, this walks them. One home (L05) serving both
//! shells: studio-shell's RiteSession and studio-tauri's MUD window.

use crate::operator::{seed_hash, Operator};

/// Which question the rite is waiting on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RiteStep {
    /// The operator's name.
    Name,
    /// The birth moon, 1-indexed at the prompt.
    Moon,
    /// The birth day, 1-indexed at the prompt.
    Day,
    /// The discipline pick (craft/calling).
    Craft,
    /// The interview is over; further strikes refuse.
    Done,
}

/// What one struck answer did to the rite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Strike {
    /// The answer held; the rite moved to the next question.
    Advance,
    /// The answer did not hold; the step is unchanged and re-asks.
    Refuse(String),
    /// The final answer held; the rite is complete.
    Complete(RiteOutcome),
}

/// The completed interview: everything `Operator::birth_with_discipline`
/// needs, plus the interview's own fnv1a seed (spec: "fnv1a seed from
/// interview" — `seed_hash` is the one seed home, operator.rs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RiteOutcome {
    /// The name the operator carries, exactly as struck.
    pub name: String,
    /// 0-indexed, `< moon_count` — the same seat the sky's set_moon takes.
    pub moon: u8,
    /// 0-indexed, `< day_count`.
    pub day: u8,
    /// 0-indexed discipline pick, `< Operator::DISCIPLINE_CHOICE_MAX`.
    pub choice: u8,
}

impl RiteOutcome {
    /// The interview's deterministic seed — same answers, same world, forever.
    pub fn seed(&self) -> u64 {
        seed_hash(&[
            self.name.as_bytes(),
            &[self.moon],
            &[self.day],
            &[self.choice],
        ])
    }

    /// Birth the operator this rite named. `None` is unreachable for a
    /// machine-produced outcome (every field was refused into range first).
    pub fn operator(&self) -> Option<Operator> {
        Operator::birth_with_discipline(&self.name, self.moon, self.day, self.choice)
    }

    /// Derive the operator's natal star index from the outcome seed.
    pub fn natal_star_idx(&self) -> usize {
        (self.seed() % 16) as usize
    }

    /// Derive the operator's natal Camelot harmonic key.
    pub fn natal_key(&self) -> forge_harmonics::CamelotKey {
        forge_harmonics::CamelotKey::from_star_idx(self.natal_star_idx())
            .unwrap_or(forge_harmonics::CamelotKey::DEFAULT_8A)
    }
}

/// The interview walker (distinct from the cart's authored
/// `forge_cart_v3::npe::BirthRite`, which is the data it walks — L05):
/// owns the cart-authored counts and craft choices so the
/// glass never re-authors them (the 2026-08-25 spine finding: the cart
/// authored the whole pick and NOTHING read it).
pub struct RiteWalk {
    step: RiteStep,
    moon_count: u8,
    day_count: u8,
    craft_prompt_word: String,
    craft_choices: Vec<(String, String)>,
    name: String,
    moon: Option<u8>,
    day: Option<u8>,
}

impl RiteWalk {
    /// A rite over the cart's authored `BirthRite` (npe.ironroot.ron,
    /// parsed at `forge-cart-v3/src/npe.rs:82`).
    pub fn from_cart(rite: &forge_cart_v3::npe::BirthRite) -> Self {
        Self {
            step: RiteStep::Name,
            moon_count: rite.moon_count.max(1),
            day_count: rite.day_count.max(1),
            craft_prompt_word: rite.craft_pick.prompt_word.clone(),
            craft_choices: rite.craft_pick.choices.clone(),
            name: String::new(),
            moon: None,
            day: None,
        }
    }

    /// Which question the rite is waiting on right now.
    pub fn step(&self) -> RiteStep {
        self.step
    }

    /// The moon already struck, if the rite has passed that question — the
    /// live sky binding reads this the frame it appears.
    pub fn struck_moon(&self) -> Option<u8> {
        self.moon
    }

    /// The question the rite is asking right now (voice: 2dak revenge tone +
    /// fae-overlay debt law — tolls, ledgers, wells that remember).
    pub fn prompt(&self) -> String {
        match self.step {
            RiteStep::Name => String::from(
                "the ledger opens. every debt in this world begins with a name.\r\nwhat name do you carry?",
            ),
            RiteStep::Moon => format!(
                "thirteen moons keep the tolls, and one of them kept yours.\r\nunder which moon were you struck? (1-{})",
                self.moon_count
            ),
            RiteStep::Day => format!(
                "a moon holds {} days, and the wells remember every one.\r\non which day did the water first repeat you? (1-{})",
                self.day_count, self.day_count
            ),
            RiteStep::Craft => {
                let mut p = format!(
                    "eight fires stand at the edge of the world. seven may be sworn.\r\nthe eighth is not yours to hold.\r\nwhich {} do you kneel to?",
                    self.craft_prompt_word
                );
                for (i, (word, _)) in self.craft_choices.iter().enumerate() {
                    p.push_str(&format!("\r\n  {} {}", i + 1, word));
                }
                p
            }
            RiteStep::Done => String::from("the rite is complete. the bell has your name now."),
        }
    }

    /// Step back one question, unstriking that answer — a mistake is not a
    /// debt. Returns `false` at the first question or after completion (a
    /// tolled bell cannot be untolled).
    pub fn back(&mut self) -> bool {
        match self.step {
            RiteStep::Name | RiteStep::Done => false,
            RiteStep::Moon => {
                self.name.clear();
                self.step = RiteStep::Name;
                true
            }
            RiteStep::Day => {
                self.moon = None;
                self.step = RiteStep::Moon;
                true
            }
            RiteStep::Craft => {
                self.day = None;
                self.step = RiteStep::Day;
                true
            }
        }
    }

    /// Strike one answer against the current question. A refusal never
    /// advances the step; the eighth craft pick refuses by the identity law
    /// (`Operator::DISCIPLINE_CHOICE_MAX` — bit 7 would alias onto anchor 0).
    pub fn strike(&mut self, answer: &str) -> Strike {
        let answer = answer.trim();
        match self.step {
            RiteStep::Name => {
                if answer.is_empty() {
                    return Strike::Refuse(String::from(
                        "the ledger refuses a blank line. a nameless debt cannot pass.",
                    ));
                }
                self.name = answer.to_string();
                self.step = RiteStep::Moon;
                Strike::Advance
            }
            RiteStep::Moon => match parse_pick(answer, self.moon_count) {
                Some(m) => {
                    self.moon = Some(m);
                    self.step = RiteStep::Day;
                    Strike::Advance
                }
                None => Strike::Refuse(format!(
                    "no such moon hangs here; the sky keeps {}. strike 1-{}.",
                    self.moon_count, self.moon_count
                )),
            },
            RiteStep::Day => match parse_pick(answer, self.day_count) {
                Some(d) => {
                    self.day = Some(d);
                    self.step = RiteStep::Craft;
                    Strike::Advance
                }
                None => Strike::Refuse(format!(
                    "no moon carries that day; {} is the whole of it. strike 1-{}.",
                    self.day_count, self.day_count
                )),
            },
            RiteStep::Craft => {
                let idx = parse_pick(answer, self.craft_choices.len() as u8).or_else(|| {
                    self.craft_choices
                        .iter()
                        .position(|(w, _)| w.eq_ignore_ascii_case(answer))
                        .map(|i| i as u8)
                });
                let Some(choice) = idx else {
                    return Strike::Refuse(format!(
                        "strike a {} by number or word.",
                        self.craft_prompt_word
                    ));
                };
                if choice >= Operator::DISCIPLINE_CHOICE_MAX {
                    return Strike::Refuse(String::from(
                        "that calling cannot anchor a world of its own; choose another.",
                    ));
                }
                self.step = RiteStep::Done;
                Strike::Complete(RiteOutcome {
                    name: std::mem::take(&mut self.name),
                    moon: self.moon.expect("moon struck before craft"),
                    day: self.day.expect("day struck before craft"),
                    choice,
                })
            }
            RiteStep::Done => Strike::Refuse(String::from(
                "the bell has already tolled; a rite strikes once.",
            )),
        }
    }
}

/// `1..=count` (or a bare digit string) to 0-indexed; anything else is `None`.
fn parse_pick(answer: &str, count: u8) -> Option<u8> {
    let n: u8 = answer.parse().ok()?;
    (n >= 1 && n <= count).then(|| n - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cart_rite() -> forge_cart_v3::npe::BirthRite {
        forge_cart_v3::npe::BirthRite {
            moon_count: 13,
            day_count: 28,
            calendar_word: String::new(),
            craft_pick: forge_cart_v3::npe::CraftPick {
                prompt_word: String::from("calling"),
                choice_count: 8,
                choices: [
                    "edge", "weight", "flow", "ring", "grain", "ember", "hollow", "veil",
                ]
                .iter()
                .map(|w| (w.to_string(), format!("{w} reading")))
                .collect(),
            },
            hidden_account_dealt: true,
            seed_console_hex: false,
        }
    }

    fn walk_to(rite: &mut RiteWalk, answers: &[&str]) -> Option<Strike> {
        answers.iter().map(|a| rite.strike(a)).last()
    }

    #[test]
    fn the_full_rite_completes_and_births() {
        let mut r = RiteWalk::from_cart(&cart_rite());
        let last = walk_to(&mut r, &["Morrow", "5", "13", "3"]).unwrap();
        let Strike::Complete(out) = last else {
            panic!("rite must complete: {last:?}");
        };
        assert_eq!((out.moon, out.day, out.choice), (4, 12, 2));
        assert!(out.operator().is_some(), "a completed rite always births");
        assert_eq!(r.step(), RiteStep::Done);
    }

    #[test]
    fn the_struck_moon_surfaces_the_frame_it_lands() {
        let mut r = RiteWalk::from_cart(&cart_rite());
        assert_eq!(r.struck_moon(), None);
        r.strike("Morrow");
        assert_eq!(r.struck_moon(), None, "not before the moon question");
        r.strike("7");
        assert_eq!(r.struck_moon(), Some(6), "set_moon's 0-indexed seat");
    }

    #[test]
    fn refusals_hold_the_step_and_reask() {
        let mut r = RiteWalk::from_cart(&cart_rite());
        assert!(matches!(r.strike("  "), Strike::Refuse(_)));
        assert_eq!(r.step(), RiteStep::Name);
        r.strike("Morrow");
        assert!(matches!(r.strike("14"), Strike::Refuse(_)), "14th moon of 13");
        assert!(matches!(r.strike("0"), Strike::Refuse(_)), "moons are 1-indexed");
        assert_eq!(r.step(), RiteStep::Moon);
    }

    #[test]
    fn the_eighth_calling_refuses_by_the_identity_law() {
        let mut r = RiteWalk::from_cart(&cart_rite());
        walk_to(&mut r, &["Morrow", "5", "13"]);
        assert!(matches!(r.strike("8"), Strike::Refuse(_)));
        assert!(matches!(r.strike("veil"), Strike::Refuse(_)), "by word too");
        assert_eq!(r.step(), RiteStep::Craft, "the rite re-asks, never folds");
        assert!(matches!(r.strike("7"), Strike::Complete(_)), "the seventh holds");
    }

    #[test]
    fn back_unstrikes_one_answer_and_the_rite_reasks() {
        let mut r = RiteWalk::from_cart(&cart_rite());
        assert!(!r.back(), "the first question has nothing behind it");
        walk_to(&mut r, &["Morrow", "5", "13"]);
        assert_eq!(r.step(), RiteStep::Craft);
        assert!(r.back());
        assert_eq!(r.step(), RiteStep::Day);
        assert!(r.back());
        assert_eq!(r.step(), RiteStep::Moon);
        assert_eq!(r.struck_moon(), None, "the moon was unstruck");
        let last = walk_to(&mut r, &["7", "13", "3"]).unwrap();
        let Strike::Complete(out) = last else { panic!("re-walk must complete") };
        assert_eq!(out.moon, 6, "the corrected moon holds");
        let mut done = RiteWalk::from_cart(&cart_rite());
        walk_to(&mut done, &["Morrow", "5", "13", "3"]);
        assert!(!done.back(), "a tolled bell cannot be untolled");
    }

    #[test]
    fn a_craft_word_strikes_like_its_number() {
        let mut r1 = RiteWalk::from_cart(&cart_rite());
        let by_num = walk_to(&mut r1, &["Morrow", "5", "13", "3"]).unwrap();
        let mut r2 = RiteWalk::from_cart(&cart_rite());
        let by_word = walk_to(&mut r2, &["Morrow", "5", "13", "flow"]).unwrap();
        assert_eq!(by_num, by_word);
    }

    #[test]
    fn same_answers_same_seed_different_answers_different_seed() {
        let out = |day: &str| {
            let mut r = RiteWalk::from_cart(&cart_rite());
            match walk_to(&mut r, &["Morrow", "5", day, "3"]).unwrap() {
                Strike::Complete(o) => o.seed(),
                s => panic!("must complete: {s:?}"),
            }
        };
        assert_eq!(out("13"), out("13"));
        assert_ne!(out("13"), out("14"));
    }
}
