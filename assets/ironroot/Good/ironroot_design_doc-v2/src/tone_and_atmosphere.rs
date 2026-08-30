pub struct ToneAndAtmosphere {
    pub core_concept: String,
    pub aesthetic_summary: String,
    pub surface_layer: Vec<String>,
    pub back_layer: Vec<String>,
    pub exclusions: Vec<String>,
}

pub fn get_tone_and_atmosphere() -> ToneAndAtmosphere {
    ToneAndAtmosphere {
        core_concept: "A beautiful world with machinery behind the wallpaper.".to_string(),
        aesthetic_summary: "Disney silhouette, legal-horror skeleton, acoustic dread nervous system.".to_string(),
        surface_layer: vec![
            "fairytale warmth".to_string(),
            "music, lanterns, orchards, old taverns".to_string(),
            "strange courts, talking rivers, ritual etiquette".to_string(),
            "beauty with small wrongnesses".to_string(),
            "wonder first".to_string(),
        ],
        back_layer: vec![
            "psychological pressure".to_string(),
            "acoustic horror".to_string(),
            "coercive systems".to_string(),
            "identity erasure".to_string(),
            "social traps".to_string(),
            "grief being converted into utility".to_string(),
            "the player slowly realizing the world is processing people".to_string(),
        ],
        exclusions: vec![
            "gore-forward".to_string(),
            "torture-porn".to_string(),
            "edgy".to_string(),
        ],
    }
}
