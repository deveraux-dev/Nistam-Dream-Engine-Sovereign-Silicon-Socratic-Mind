pub struct Interactions {
    pub high_resonance: Option<String>,
    pub high_clarity: Option<String>,
    pub high_tarnish: Option<String>,
    pub polished_paths: Option<Vec<String>>,
    pub plain_song: String,
}

pub struct Encounter {
    pub name: String,
    pub front_read: String,
    pub deeper_read: String,
    pub interactions: Interactions,
}

pub struct WorldEncounters {
    pub encounters: Vec<Encounter>,
}

pub fn get_world_encounters() -> WorldEncounters {
    WorldEncounters {
        encounters: vec![
            Encounter {
                name: "Singing Flowers".to_string(),
                front_read: "Cute flowers hum along when the Bard plays. Very fairytale.".to_string(),
                deeper_read: "Each flower is repeating the last note of someone erased nearby. They are not singing. They are stuck.".to_string(),
                interactions: Interactions {
                    high_resonance: Some("can tune them.".to_string()),
                    high_clarity: Some("can identify the erased name pattern.".to_string()),
                    high_tarnish: Some("can harvest them.".to_string()),
                    polished_paths: None,
                    plain_song: "sits and plays with them until the loop changes. No extraction. No puzzle solved. Later, an NPC remembers one extra detail. Small mercy. Huge consequence.".to_string(),
                },
            },
            Encounter {
                name: "The Friendly Puppet Show".to_string(),
                front_read: "A village has a charming puppet theatre for children. Bright cloth. Wooden saints. Little songs. Harmless.".to_string(),
                deeper_read: "The theatre is a civic memory audit. Children are taught which names no longer belong in family stories. Normalized erasure.".to_string(),
                interactions: Interactions {
                    high_resonance: None,
                    high_clarity: None,
                    high_tarnish: None,
                    polished_paths: None,
                    plain_song: "The Bard joins the song, then gently sings one missing name in the same melody. Children repeat it. The system cannot punish them all without revealing itself.".to_string(),
                },
            },
            Encounter {
                name: "The Polite Fae Banquet".to_string(),
                front_read: "Gorgeous feast. Crystal fruit. Moonlit etiquette. Everyone speaks beautifully.".to_string(),
                deeper_read: "Every compliment is a narrowing contract. Every toast assigns debt. Every dance step confirms consent.".to_string(),
                interactions: Interactions {
                    high_resonance: None,
                    high_clarity: None,
                    high_tarnish: None,
                    polished_paths: Some(vec![
                        "out-lawyering the contract".to_string(),
                        "out-resonating the music".to_string(),
                        "lying through subtext".to_string(),
                        "intimidating servants".to_string(),
                    ]),
                    plain_song: "The Bard simply admits: 'I do not know the proper answer. I do not want to insult you. I came because someone is missing.' It disrupts the game by offering honest incompleteness.".to_string(),
                },
            },
        ],
    }
}
