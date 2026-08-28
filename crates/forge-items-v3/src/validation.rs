use crate::item::{Item, PartKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateError {
    MissingRequiredPart(&'static str),
    TooManySockets(u8),
    NoGameplayConsequence,
    InvalidDurability,
    StatOverflowRisk,
    EmptyTags,
}

pub fn validate_item(item: &Item) -> Result<(), Vec<GateError>> {
    let mut errors = Vec::new();

    for required in [PartKind::Blade, PartKind::Guard, PartKind::Grip, PartKind::Pommel] {
        if !item.parts.iter().any(|p| p.kind == required) {
            let name = match required {
                PartKind::Blade => "blade",
                PartKind::Guard => "guard",
                PartKind::Grip => "grip",
                PartKind::Pommel => "pommel",
                _ => "part",
            };
            errors.push(GateError::MissingRequiredPart(name));
        }
    }

    if item.sockets > 2 {
        errors.push(GateError::TooManySockets(item.sockets));
    }
    if item.damage.base == 0 && item.defense.physical == 0 && item.stats.vigor == 0 {
        errors.push(GateError::NoGameplayConsequence);
    }
    if item.durability_current > item.durability_max || item.durability_max == 0 {
        errors.push(GateError::InvalidDurability);
    }
    if item.tags.is_empty() {
        errors.push(GateError::EmptyTags);
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::{ForgeConfig, ItemForge};

    #[test]
    fn generated_sword_passes_gates() {
        let mut forge = ItemForge::new(ForgeConfig::default());
        let item = forge.generate_sword(42);
        assert!(validate_item(&item).is_ok());
        assert!(item.sockets <= 2);
    }
}
