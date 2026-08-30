pub struct NameCandidate {
    pub name: String,
    pub feel: String,
}

pub struct PathIdentity {
    pub route_name: String,
    pub ending_name: String,
    pub name_candidates: Vec<NameCandidate>,
    pub core_traits: Vec<String>,
    pub design_philosophy: String,
}

pub struct ActionMapping {
    pub player_action: String,
    pub presence_meaning: String,
}

pub struct Mechanics {
    pub primary_variable: String,
    pub lore_facing_variables: Vec<String>,
    pub trigger_condition: String,
    pub action_mapping: Vec<ActionMapping>,
}

pub struct BardPathMechanics {
    pub path_identity: PathIdentity,
    pub mechanics: Mechanics,
}

pub fn get_bard_path_mechanics() -> BardPathMechanics {
    BardPathMechanics {
        path_identity: PathIdentity {
            route_name: "The Plain Song".to_string(),
            ending_name: "The Song That Stayed".to_string(),
            name_candidates: vec![
                NameCandidate { name: "The Open Chord".to_string(), feel: "gentle, musical, hopeful".to_string() },
                NameCandidate { name: "The Plain Song".to_string(), feel: "humble, folk-tale".to_string() },
                NameCandidate { name: "The Hearth Tune".to_string(), feel: "warm, human".to_string() },
                NameCandidate { name: "The Unpolished Song".to_string(), feel: "imperfect but sincere".to_string() },
                NameCandidate { name: "The Kindly Discord".to_string(), feel: "strange but benevolent".to_string() },
                NameCandidate { name: "The Song That Stayed".to_string(), feel: "strongest ending energy".to_string() },
                NameCandidate { name: "The Honest Note".to_string(), feel: "direct, clean".to_string() },
            ],
            core_traits: vec![
                "plays with feeling before technique".to_string(),
                "listens even when he cannot decode everything".to_string(),
                "apologizes when he hurts someone".to_string(),
                "returns after failure".to_string(),
                "remembers small people".to_string(),
                "comforts before optimizing".to_string(),
                "accepts partial good".to_string(),
                "does not turn every wound into a weapon".to_string(),
                "does not need the song to be impressive for it to be true".to_string(),
            ],
            design_philosophy: "Presence without extraction. Rewards care, not bad play.".to_string(),
        },
        mechanics: Mechanics {
            primary_variable: "presence_q".to_string(),
            lore_facing_variables: vec![
                "hearth_presence_q".to_string(),
                "plain_song_q".to_string(),
                "witnessed_kindness_q".to_string(),
                "unclaimed_grief_q".to_string(),
            ],
            trigger_condition: "Rises when the player chooses sincere, non-extractive actions.".to_string(),
            action_mapping: vec![
                ActionMapping { player_action: "Plays a simple tune at a grave".to_string(), presence_meaning: "Remembrance without reward".to_string() },
                ActionMapping { player_action: "Comforts an NPC after failing to save someone".to_string(), presence_meaning: "Staying with consequence".to_string() },
                ActionMapping { player_action: "Uses a damaged instrument instead of replacing it".to_string(), presence_meaning: "Loyalty to history".to_string() },
                ActionMapping { player_action: "Accepts a partial rescue".to_string(), presence_meaning: "Refusing perfectionism".to_string() },
                ActionMapping { player_action: "Tells the truth awkwardly".to_string(), presence_meaning: "Honesty over polish".to_string() },
                ActionMapping { player_action: "Declines a power upgrade born from suffering".to_string(), presence_meaning: "Refusing extraction".to_string() },
                ActionMapping { player_action: "Returns to a village after it changed".to_string(), presence_meaning: "Witnessing damage".to_string() },
                ActionMapping { player_action: "Lets silence sit in dialogue".to_string(), presence_meaning: "Not filling pain with performance".to_string() },
            ],
        },
    }
}
