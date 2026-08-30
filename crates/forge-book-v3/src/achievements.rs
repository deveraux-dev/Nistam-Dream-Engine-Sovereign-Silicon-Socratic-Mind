//! Chapter completion track for building a game engine, mirroring
//! Book_13_Chapters_Organized.md's 13 domains. Drained from Desktop
//! (PULL-BOARD DUEL-777 NEXT: "drain Desktop manuscripts into canon") — was
//! complete + tested, 0 live caller. Live caller: state_board.rs CHAPTER 0.

use serde::{Deserialize, Serialize};

/// Chapter completion track for building a game engine.
/// Based on 13 Domains: Foundation → Interesting progression.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterAchievement {
    /// Unique identifier for this achievement (e.g., "ch01_safety").
    pub id: String,
    /// Chapter number this achievement corresponds to (1-13, or 0 for meta).
    pub number: u8,
    /// Display name of the achievement (e.g., "Mercy & Iron").
    pub name: String,
    /// Domain category from the 13-domain curriculum (e.g., "Safety, Security & Ethics").
    pub domain: String,
    /// Detailed description of what this achievement represents.
    pub description: String,
    /// SVG path icon for visual representation of the achievement.
    pub icon: String,
    /// Whether this achievement has been earned.
    pub earned: bool,
    /// Current progress string (e.g., "EARNED" or a task list).
    pub progress: String,
    /// Achievement tier level (Bronze/Silver/Gold/Platinum).
    pub tier: AchievementTier,
}

/// Achievement tier levels from foundation to mastery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AchievementTier {
    /// Foundation tier: core systems (chapters 1-4).
    Bronze,
    /// Systems tier: integrated systems (chapters 5-8).
    Silver,
    /// Intelligence tier: smart systems (chapters 9-12).
    Gold,
    /// Mastery tier: complete engine (chapter 13).
    Platinum,
}

/// Evaluate engine build progress across 13 chapters.
pub fn evaluate_engine_achievements(
    completed_chapters: &[u8],
) -> Vec<ChapterAchievement> {
    let total_complete = completed_chapters.len();

    vec![
        // ─── Bronze tier: Foundation ────────────────────────────────────────
        ChapterAchievement {
            id: "ch01_safety".into(),
            number: 1,
            name: "Mercy & Iron".into(),
            domain: "Safety, Security & Ethics".into(),
            description: "Establish ethics framework, threat models, and safety gates".into(),
            icon: "M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944".into(),
            earned: completed_chapters.contains(&1),
            progress: if completed_chapters.contains(&1) {
                "EARNED".into()
            } else {
                "Define: ethics, threat model, mercy TTL, safety gates".into()
            },
            tier: AchievementTier::Bronze,
        },
        ChapterAchievement {
            id: "ch02_architecture".into(),
            number: 2,
            name: "Blueprint Laid".into(),
            domain: "Architecture & Design Theory".into(),
            description: "ADRs, semantic layer, convergence canon, primitives defined".into(),
            icon: "M19.428 15.428a2 2 0 00-1.022-.547l-2.387-.477".into(),
            earned: completed_chapters.contains(&2),
            progress: if completed_chapters.contains(&2) {
                "EARNED".into()
            } else {
                "Document: 4 ADRs, semantic 6-axis, primitives".into()
            },
            tier: AchievementTier::Bronze,
        },
        ChapterAchievement {
            id: "ch03_rendering".into(),
            number: 3,
            name: "Pixels & Light".into(),
            domain: "Rendering & Graphics".into(),
            description: "Renderer engine, frustum culling, material system, lighting pipeline".into(),
            icon: "M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4".into(),
            earned: completed_chapters.contains(&3),
            progress: if completed_chapters.contains(&3) {
                "EARNED".into()
            } else {
                "Build: renderer arch, shadow maps, materials, LOD".into()
            },
            tier: AchievementTier::Bronze,
        },
        ChapterAchievement {
            id: "ch04_audio".into(),
            number: 4,
            name: "Sonic Forge".into(),
            domain: "Audio Systems & Music".into(),
            description: "Audio mixer, DSP, synth engine, real-time music system".into(),
            icon: "M9 19V5m0 0a9 9 0 0118 0m0 0V5m0 0a9 9 0 00-18 0".into(),
            earned: completed_chapters.contains(&4),
            progress: if completed_chapters.contains(&4) {
                "EARNED".into()
            } else {
                "Implement: audio IO, DSP, SynthXML, resonance".into()
            },
            tier: AchievementTier::Bronze,
        },

        // ─── Silver tier: Systems ───────────────────────────────────────────
        ChapterAchievement {
            id: "ch05_assets".into(),
            number: 5,
            name: "Foundry Open".into(),
            domain: "Asset Pipeline & Creation".into(),
            description: "Sprite pipeline, background removal, GLB simplify, multi-format import".into(),
            icon: "M13 10V3L4 14h7v7l9-11h-7z".into(),
            earned: completed_chapters.contains(&5),
            progress: if completed_chapters.contains(&5) {
                "EARNED".into()
            } else {
                "Wire: asset import, sprite split, format converters".into()
            },
            tier: AchievementTier::Silver,
        },
        ChapterAchievement {
            id: "ch06_worldbuild".into(),
            number: 6,
            name: "The Cartographer".into(),
            domain: "World Building & Cartography".into(),
            description: "Worldgen, navmesh, spatial indexing, voxel architecture".into(),
            icon: "M9 20l-5.447-2.724A1 1 0 003 16.382V5.618a1 1 0 011.553-.894L9 7m0 13l6-3m-6 3V7m6 3l5.447-2.724A1 1 0 0021 5.618v10.764".into(),
            earned: completed_chapters.contains(&6),
            progress: if completed_chapters.contains(&6) {
                "EARNED".into()
            } else {
                "Create: prime sieve, spatial index, navmesh".into()
            },
            tier: AchievementTier::Silver,
        },
        ChapterAchievement {
            id: "ch07_gamelogic".into(),
            number: 7,
            name: "Mechanics Master".into(),
            domain: "Game Logic & Mechanics".into(),
            description: "Physics engine, item system, combat, game architecture".into(),
            icon: "M14.828 14.828a4 4 0 01-5.656 0M9 10h.01M15 10h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z".into(),
            earned: completed_chapters.contains(&7),
            progress: if completed_chapters.contains(&7) {
                "EARNED".into()
            } else {
                "Implement: physics, items, combat logic, interaction".into()
            },
            tier: AchievementTier::Silver,
        },
        ChapterAchievement {
            id: "ch08_animation".into(),
            number: 8,
            name: "Motion Captured".into(),
            domain: "Animation & Motion".into(),
            description: "Skeletal animation, spring physics, particles, transitions".into(),
            icon: "M14.828 14.828a4 4 0 01-5.656 0M9 10h.01M15 10h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z".into(),
            earned: completed_chapters.contains(&8),
            progress: if completed_chapters.contains(&8) {
                "EARNED".into()
            } else {
                "Build: skeletal rig, spring anim, particle system".into()
            },
            tier: AchievementTier::Silver,
        },

        // ─── Gold tier: Intelligence ────────────────────────────────────────
        ChapterAchievement {
            id: "ch09_ui".into(),
            number: 9,
            name: "The Interface".into(),
            domain: "UI, Dialogue & Interaction".into(),
            description: "Native UI, dialogue authoring, visual editor, scripting".into(),
            icon: "M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100 4m0-4a2 2 0 110 4".into(),
            earned: completed_chapters.contains(&9),
            progress: if completed_chapters.contains(&9) {
                "EARNED".into()
            } else {
                "Create: UI system, dialogue tree, visual tools".into()
            },
            tier: AchievementTier::Gold,
        },
        ChapterAchievement {
            id: "ch10_creation".into(),
            number: 10,
            name: "The Maker".into(),
            domain: "Creation Engine & Proceduralism".into(),
            description: "Procedural generation, dream worker, deterministic recording".into(),
            icon: "M19.428 15.428a2 2 0 00-1.022-.547l-2.387-.477a6 6 0 00-3.86.517".into(),
            earned: completed_chapters.contains(&10),
            progress: if completed_chapters.contains(&10) {
                "EARNED".into()
            } else {
                "Implement: proceduralism, creation API, alchemy".into()
            },
            tier: AchievementTier::Gold,
        },
        ChapterAchievement {
            id: "ch11_aiml".into(),
            number: 11,
            name: "Semantic Mind".into(),
            domain: "Semantic AI & ML Systems".into(),
            description: "Semantic gates, vision, ML integration, QA engine".into(),
            icon: "M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1".into(),
            earned: completed_chapters.contains(&11),
            progress: if completed_chapters.contains(&11) {
                "EARNED".into()
            } else {
                "Wire: semantic layer, vision morph, QA system".into()
            },
            tier: AchievementTier::Gold,
        },
        ChapterAchievement {
            id: "ch12_multiplayer".into(),
            number: 12,
            name: "Shared Worlds".into(),
            domain: "Collaborative World Building".into(),
            description: "Multiplayer systems, headless server, IPC architecture".into(),
            icon: "M18 9v3m0 0v3m0-3h3m0 0h3m-6-3a9 9 0 11-18 0 9 9 0 0118 0z".into(),
            earned: completed_chapters.contains(&12),
            progress: if completed_chapters.contains(&12) {
                "EARNED".into()
            } else {
                "Build: multiplayer, server arch, networking".into()
            },
            tier: AchievementTier::Gold,
        },

        // ─── Platinum tier: Mastery ────────────────────────────────────────
        ChapterAchievement {
            id: "ch13_advanced".into(),
            number: 13,
            name: "Engine Complete".into(),
            domain: "Advanced Topics & Polish".into(),
            description: "Card games, game tools, final polish, release-ready".into(),
            icon: "M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z".into(),
            earned: completed_chapters.contains(&13),
            progress: if completed_chapters.contains(&13) {
                "EARNED".into()
            } else {
                "Polish: CCG system, tools, documentation, release".into()
            },
            tier: AchievementTier::Platinum,
        },

        // ─── Meta achievements ──────────────────────────────────────────────
        ChapterAchievement {
            id: "foundation_laid".into(),
            number: 0,
            name: "Foundation Laid".into(),
            domain: "Meta: Chapters 1-4".into(),
            description: "Core systems ready — safety, architecture, rendering, audio".into(),
            icon: "M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4".into(),
            earned: completed_chapters.iter().filter(|&&c| c >= 1 && c <= 4).count() >= 4,
            progress: format!("{}/4 foundation chapters",
                completed_chapters.iter().filter(|&&c| c >= 1 && c <= 4).count()),
            tier: AchievementTier::Bronze,
        },
        ChapterAchievement {
            id: "systems_integrated".into(),
            number: 0,
            name: "Systems Integrated".into(),
            domain: "Meta: Chapters 5-8".into(),
            description: "Major systems wired — assets, world, logic, animation".into(),
            icon: "M13 10V3L4 14h7v7l9-11h-7z".into(),
            earned: completed_chapters.iter().filter(|&&c| c >= 5 && c <= 8).count() >= 4,
            progress: format!("{}/4 systems chapters",
                completed_chapters.iter().filter(|&&c| c >= 5 && c <= 8).count()),
            tier: AchievementTier::Silver,
        },
        ChapterAchievement {
            id: "intelligence_wired".into(),
            number: 0,
            name: "Intelligence Wired".into(),
            domain: "Meta: Chapters 9-12".into(),
            description: "Smart systems live — UI, creation, AI, multiplayer".into(),
            icon: "M19.428 15.428a2 2 0 00-1.022-.547l-2.387-.477".into(),
            earned: completed_chapters.iter().filter(|&&c| c >= 9 && c <= 12).count() >= 4,
            progress: format!("{}/4 intelligence chapters",
                completed_chapters.iter().filter(|&&c| c >= 9 && c <= 12).count()),
            tier: AchievementTier::Gold,
        },
        ChapterAchievement {
            id: "sovereign_master".into(),
            number: 0,
            name: "Sovereign Master".into(),
            domain: "Meta: All 13 Chapters".into(),
            description: "Engine complete — independent, polished, ready to ship".into(),
            icon: "M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944".into(),
            earned: total_complete >= 13,
            progress: format!("{}/13 chapters complete", total_complete),
            tier: AchievementTier::Platinum,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_chapters_complete() {
        let achievements = evaluate_engine_achievements(&[]);
        let sovereign = achievements.iter().find(|a| a.id == "sovereign_master").unwrap();
        assert!(!sovereign.earned);
        assert_eq!(sovereign.progress, "0/13 chapters complete");
    }

    #[test]
    fn test_foundation_tier() {
        let achievements = evaluate_engine_achievements(&[1, 2, 3, 4]);
        let foundation = achievements.iter().find(|a| a.id == "foundation_laid").unwrap();
        assert!(foundation.earned);
    }

    #[test]
    fn test_all_chapters_complete() {
        let all_chapters = (1..=13).collect::<Vec<_>>();
        let achievements = evaluate_engine_achievements(&all_chapters);
        let sovereign = achievements.iter().find(|a| a.id == "sovereign_master").unwrap();
        assert!(sovereign.earned);
        assert_eq!(sovereign.progress, "13/13 chapters complete");
    }
}
