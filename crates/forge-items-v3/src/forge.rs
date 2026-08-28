use crate::catalog::{BLADES, GRIPS, GUARDS, POMMELS, RUNES};
use crate::item::*;
use crate::rng::XorShift64;

#[derive(Debug, Clone, Copy)]
pub struct ForgeConfig {
    pub max_sockets: u8,
    pub base_level_req: u8,
    pub base_durability: u16,
}

impl Default for ForgeConfig {
    fn default() -> Self {
        Self { max_sockets: 2, base_level_req: 1, base_durability: 100 }
    }
}

#[derive(Debug, Clone)]
pub struct ItemForge {
    config: ForgeConfig,
}

impl ItemForge {
    pub fn new(config: ForgeConfig) -> Self {
        assert!(config.max_sockets <= 2, "schema item depth limit is 0-2 sockets");
        Self { config }
    }

    pub fn generate_sword(&mut self, seed: u64) -> Item {
        let mut rng = XorShift64::new(seed);
        let blade = BLADES[rng.range(BLADES.len())];
        let guard = GUARDS[rng.range(GUARDS.len())];
        let grip = GRIPS[rng.range(GRIPS.len())];
        let pommel = POMMELS[rng.range(POMMELS.len())];

        let socket_count = (rng.range((self.config.max_sockets + 1) as usize)) as u8;
        let mut parts = vec![pommel, grip, guard, blade];
        for _ in 0..socket_count {
            parts.push(RUNES[rng.range(RUNES.len())]);
        }

        self.assemble(seed, parts, socket_count)
    }

    fn assemble(&self, seed: u64, parts: Vec<Part>, socket_count: u8) -> Item {
        let mut stats = ItemStats::ZERO;
        let mut damage_acc = 6i16;
        let mut weight_acc = 0u32;
        let mut tags: Vec<&'static str> = Vec::new();
        let mut primary_element = Element::Earth;
        let mut active = 0i16;
        let mut passive = 0i16;

        for p in &parts {
            stats = stats.saturating_add(p.stats);
            damage_acc += p.damage_delta;
            weight_acc += p.weight_permyriad as u32;
            primary_element = p.element;
            match p.polarity {
                GenderPolarity::Active => active += 1,
                GenderPolarity::Passive => passive += 1,
                GenderPolarity::Neutral => {}
            }
            for t in p.tags {
                if !tags.contains(t) {
                    tags.push(t);
                }
            }
        }

        let gender = if active > passive { GenderPolarity::Active } else if passive > active { GenderPolarity::Passive } else { GenderPolarity::Neutral };
        let tier = derive_tier(socket_count, damage_acc, tags.len());
        let base_damage = damage_acc.clamp(1, 255) as u8;
        let freq_byte = ((seed ^ (weight_acc as u64)) & 0xFF) as u8;
        let durability_max = self.config.base_durability + (tier as u16 * 35) + ((weight_acc / 100) as u16);
        let blade_name = parts.iter().find(|p| p.kind == PartKind::Blade).map(|p| p.name).unwrap_or("Blade");
        let guard_name = parts.iter().find(|p| p.kind == PartKind::Guard).map(|p| p.name).unwrap_or("Guard");
        let material = parts.iter().find(|p| p.kind == PartKind::Blade).map(|p| p.material).unwrap_or("IRON").to_string();

        Item {
            id: format!("wpn_proc_{:016x}", seed),
            name: derive_name(tier, blade_name, guard_name, &tags),
            slot: ItemSlot::Weapon,
            tier,
            level_req: self.config.base_level_req.saturating_add(tier * 2),
            stats,
            damage: Damage { base: base_damage, element: primary_element, freq_byte },
            defense: Defense { physical: (tier * 2).saturating_add(if tags.contains(&"parry") { 3 } else { 0 }), element_resist: vec![(primary_element, 500 + tier as i32 * 500)] },
            tags,
            material,
            gender,
            durability_current: durability_max,
            durability_max,
            sockets: socket_count,
            description: derive_description(tier, &parts),
            parts,
        }
    }
}

fn derive_tier(sockets: u8, damage: i16, tag_count: usize) -> u8 {
    let score = sockets as i16 * 2 + damage / 8 + tag_count as i16 / 3;
    match score {
        0..=2 => 0,
        3..=4 => 1,
        5..=6 => 2,
        _ => 3,
    }
}

fn derive_name(tier: u8, blade: &str, guard: &str, tags: &[&str]) -> String {
    let prefix = if tags.contains(&"bloom") { "Bloom" }
        else if tags.contains(&"darkness") { "Void" }
        else if tags.contains(&"meridian") { "Meridian" }
        else if tags.contains(&"bandit") { "Bandit" }
        else { "Worn" };
    let rank = match tier { 0 => "", 1 => " Marked", 2 => " Oath", _ => " Artifact" };
    format!("{} {} / {}{}", prefix, blade, guard, rank)
}

fn derive_description(tier: u8, parts: &[Part]) -> String {
    let rune_count = parts.iter().filter(|p| p.kind == PartKind::Rune).count();
    match (tier, rune_count) {
        (3, n) if n > 0 => "Socketed too deeply. It works, but it remembers the hand.".to_string(),
        (_, n) if n > 0 => "A practical weapon with something breathing under the wrap.".to_string(),
        _ => "Bent, useful, and not clean enough to lie.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_by_seed() {
        let mut a = ItemForge::new(ForgeConfig::default());
        let mut b = ItemForge::new(ForgeConfig::default());
        assert_eq!(a.generate_sword(999).to_json_pretty(), b.generate_sword(999).to_json_pretty());
    }

    #[test]
    fn permyriad_holds_negative_resist() {
        // RED-first proof for F1 u16→i32 fix.
        // u16 could not represent -2000; i32 must hold it without wrapping.
        let debuff: Permyriad = -2000;
        assert_eq!(debuff, -2000, "resist debuff -2000 must be representable");
        assert!(debuff < 0, "negative resist is valid game state");
        // Also verify positive stays intact.
        let boost: Permyriad = 5000;
        assert_eq!(boost, 5000);
    }
}
