pub struct KeyDesignLaw {
    pub rule: String,
    pub ironroot_machinery: String,
    pub plain_song_victory: Vec<String>,
}

pub struct ToneGuardrails {
    pub avoid: Vec<String>,
    pub prefer: Vec<String>,
    pub horror_source: String,
    pub hope_source: String,
}

pub struct DesignGuidelines {
    pub key_design_law: KeyDesignLaw,
    pub tone_guardrails: ToneGuardrails,
}

pub fn get_design_guidelines() -> DesignGuidelines {
    DesignGuidelines {
        key_design_law: KeyDesignLaw {
            rule: "The player does not overcome the world by being exceptional. The player survives the world by remaining relational.".to_string(),
            ironroot_machinery: "Wants isolated inputs, clean records, optimized choices, and convertible pain.".to_string(),
            plain_song_victory: vec![
                "witness".to_string(), "return".to_string(), "apology".to_string(), "memory".to_string(),
                "care".to_string(), "partial repair".to_string(), "shared song".to_string(),
                "grief that does not become a commodity".to_string(),
            ],
        },
        tone_guardrails: ToneGuardrails {
            avoid: vec![
                "terrible Bard".to_string(), "incompetent".to_string(), "wrong".to_string(),
                "failure build".to_string(), "joke route".to_string(), "gore imagery as primary horror".to_string(),
                "cruelty as spectacle".to_string(),
            ],
            prefer: vec![
                "plain".to_string(), "unpolished".to_string(), "sincere".to_string(), "weathered".to_string(),
                "open".to_string(), "small".to_string(), "human".to_string(), "stayed".to_string(),
                "remembered".to_string(), "shared".to_string(),
            ],
            horror_source: "Realizing what beautiful systems are doing to people.".to_string(),
            hope_source: "Small human acts remaining outside the machine.".to_string(),
        },
    }
}
