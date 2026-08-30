pub struct SystemicConflict {
    pub system_question: String,
    pub other_ending_answers: Vec<String>,
    pub plain_song_answer: String,
}

pub struct FinalSceneSketch {
    pub environment: String,
    pub environmental_detail: String,
    pub standard_options: Vec<String>,
    pub plain_song_option: String,
    pub resolution: String,
}

pub struct GrandEnding {
    pub ending_name: String,
    pub core_premise: String,
    pub systemic_conflict: SystemicConflict,
    pub final_scene_sketch: FinalSceneSketch,
}

pub fn get_grand_ending() -> GrandEnding {
    GrandEnding {
        ending_name: "The Song That Stayed".to_string(),
        core_premise: "He wins because his music never becomes a weapon. It remains grief, memory, and love. Not optimized. Not purified. Not monetized by the soul. Just carried.".to_string(),
        systemic_conflict: SystemicConflict {
            system_question: "What did your grief become?".to_string(),
            other_ending_answers: vec![
                "power".to_string(), "law".to_string(), "truth".to_string(), "vengeance".to_string(),
                "mastery".to_string(), "corruption".to_string(), "silence".to_string(), "refusal".to_string(),
            ],
            plain_song_answer: "'It stayed grief.' The Ironroot cannot fully process grief that remains love, because it cannot be weaponized into force, guilt, debt, law, identity, or proof.".to_string(),
        },
        final_scene_sketch: FinalSceneSketch {
            environment: "A clean, beautiful hall. Warm gold light. Children's mural on the ceiling. A soft music-box theme playing slightly too slow.".to_string(),
            environmental_detail: "Every erased name appears as a blank space in the melody.".to_string(),
            standard_options: vec![
                "sing the True Name and bind the record".to_string(),
                "break the ledger".to_string(),
                "rewrite the court-law".to_string(),
                "invert the Name-Shear".to_string(),
                "spend accumulated Guilt".to_string(),
                "weaponize the Shadow".to_string(),
                "perfect the song".to_string(),
            ],
            plain_song_option: "Play what you remember. Not the correct melody. Not complete. Not powerful.".to_string(),
            resolution: "NPCs who witnessed his presence begin filling in small pieces... The final song is not mastered. It is shared.".to_string(),
        },
    }
}
