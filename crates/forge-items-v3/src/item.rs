use core::fmt;

// F1: was u16 (correctness bug — negative values are valid, e.g. resist -2000).
pub type Permyriad = i32;
pub type Byte = u8;
pub type SignedByte = i8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemSlot {
    Weapon = 0,
    Offhand = 1,
    Head = 2,
    Chest = 3,
    Arms = 4,
    Legs = 5,
    Boots = 6,
    Accessory1 = 7,
    Accessory2 = 8,
    Sigil1 = 9,
    Sigil2 = 10,
    Relic = 11,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Element {
    Fire,
    Poison,
    Water,
    Light,
    Electric,
    Blood,
    Earth,
    Darkness,
}

impl fmt::Display for Element {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Element::Fire => "fire",
            Element::Poison => "poison",
            Element::Water => "water",
            Element::Light => "light",
            Element::Electric => "electric",
            Element::Blood => "blood",
            Element::Earth => "earth",
            Element::Darkness => "darkness",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenderPolarity {
    Active,
    Passive,
    Neutral,
}

impl fmt::Display for GenderPolarity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            GenderPolarity::Active => "active",
            GenderPolarity::Passive => "passive",
            GenderPolarity::Neutral => "neutral",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketKind {
    Pommel,
    Grip,
    Guard,
    Blade,
    Rune,
    MaterialLayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartKind {
    Pommel,
    Grip,
    Guard,
    Blade,
    Rune,
    Material,
}

#[derive(Debug, Clone, Copy)]
pub struct ItemStats {
    pub vigor: SignedByte,
    pub momentum: SignedByte,
    pub logic_depth: SignedByte,
    pub shadow_weight: SignedByte,
    pub tarnish: SignedByte,
    pub resonance: SignedByte,
    pub guilt: SignedByte,
    pub clarity: SignedByte,
}

impl ItemStats {
    pub const ZERO: Self = Self {
        vigor: 0,
        momentum: 0,
        logic_depth: 0,
        shadow_weight: 0,
        tarnish: 0,
        resonance: 0,
        guilt: 0,
        clarity: 0,
    };

    pub fn saturating_add(self, rhs: Self) -> Self {
        Self {
            vigor: self.vigor.saturating_add(rhs.vigor),
            momentum: self.momentum.saturating_add(rhs.momentum),
            logic_depth: self.logic_depth.saturating_add(rhs.logic_depth),
            shadow_weight: self.shadow_weight.saturating_add(rhs.shadow_weight),
            tarnish: self.tarnish.saturating_add(rhs.tarnish),
            resonance: self.resonance.saturating_add(rhs.resonance),
            guilt: self.guilt.saturating_add(rhs.guilt),
            clarity: self.clarity.saturating_add(rhs.clarity),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Damage {
    pub base: Byte,
    pub element: Element,
    pub freq_byte: Byte,
}

#[derive(Debug, Clone)]
pub struct Defense {
    pub physical: Byte,
    pub element_resist: Vec<(Element, Permyriad)>,
}

#[derive(Debug, Clone, Copy)]
pub struct Part {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: PartKind,
    pub socket: SocketKind,
    pub material: &'static str,
    pub element: Element,
    pub polarity: GenderPolarity,
    pub stats: ItemStats,
    pub damage_delta: i16,
    pub weight_permyriad: Permyriad,
    pub tags: &'static [&'static str],
}

#[derive(Debug, Clone)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub slot: ItemSlot,
    pub tier: u8,
    pub level_req: u8,
    pub stats: ItemStats,
    pub damage: Damage,
    pub defense: Defense,
    pub tags: Vec<&'static str>,
    pub material: String,
    pub gender: GenderPolarity,
    pub durability_current: u16,
    pub durability_max: u16,
    pub sockets: u8,
    pub description: String,
    pub parts: Vec<Part>,
}

impl Item {
    pub fn to_json_pretty(&self) -> String {
        let mut out = String::new();
        out.push_str("{\n");
        out.push_str(&format!("  \"id\": \"{}\",\n", esc(&self.id)));
        out.push_str(&format!("  \"name\": \"{}\",\n", esc(&self.name)));
        out.push_str(&format!("  \"slot\": {},\n", self.slot as u8));
        out.push_str(&format!("  \"tier\": {},\n", self.tier));
        out.push_str(&format!("  \"level_req\": {},\n", self.level_req));
        out.push_str("  \"stats\": {");
        out.push_str(&format!("\"vigor\":{},\"momentum\":{},\"logic_depth\":{},\"shadow_weight\":{},\"tarnish\":{},\"resonance\":{},\"guilt\":{},\"clarity\":{}", self.stats.vigor, self.stats.momentum, self.stats.logic_depth, self.stats.shadow_weight, self.stats.tarnish, self.stats.resonance, self.stats.guilt, self.stats.clarity));
        out.push_str("},\n");
        out.push_str(&format!("  \"damage\": {{\"base\":{},\"element\":\"{}\",\"freq_byte\":{}}},\n", self.damage.base, self.damage.element, self.damage.freq_byte));
        out.push_str(&format!("  \"defense\": {{\"physical\":{},\"element_resist\":{{", self.defense.physical));
        for (i, (el, val)) in self.defense.element_resist.iter().enumerate() {
            if i > 0 { out.push(','); }
            out.push_str(&format!("\"{}\":{}", el, val));
        }
        out.push_str("}}},\n");
        out.push_str("  \"tags\": [");
        for (i, tag) in self.tags.iter().enumerate() {
            if i > 0 { out.push_str(", "); }
            out.push_str(&format!("\"{}\"", esc(tag)));
        }
        out.push_str("],\n");
        out.push_str(&format!("  \"material\": \"{}\",\n", esc(&self.material)));
        out.push_str(&format!("  \"gender\": \"{}\",\n", self.gender));
        out.push_str(&format!("  \"durability\": {{\"current\":{},\"max\":{}}},\n", self.durability_current, self.durability_max));
        out.push_str(&format!("  \"sockets\": {},\n", self.sockets));
        out.push_str(&format!("  \"description\": \"{}\",\n", esc(&self.description)));
        out.push_str("  \"parts\": [");
        for (i, p) in self.parts.iter().enumerate() {
            if i > 0 { out.push_str(", "); }
            out.push_str(&format!("{{\"id\":\"{}\",\"name\":\"{}\",\"material\":\"{}\"}}", esc(p.id), esc(p.name), esc(p.material)));
        }
        out.push_str("]\n}");
        out
    }
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}


/// Bridge payload for forge-architecture::catalog_bridge::MeshCatalogEntry.
/// This is the typed seam between the item generator and the runtime consumer.
#[derive(Debug, Clone)]
pub struct CatalogPayload {
    pub id: String,
    pub name: String,
    pub slot: u8,
    pub tier: u8,
    pub material_tag: String,
    pub socket_count: u8,
    pub base_mass: u32,
    pub resonance_hz: u32,
    pub tags: Vec<String>,
}

impl Item {
    /// Convert to the catalog bridge payload (matches MeshCatalogEntry shape).
    pub fn to_catalog_payload(&self) -> CatalogPayload {
        let mass = self.parts.iter().map(|p| p.weight_permyriad as u32).sum::<u32>();
        // resonance_hz: material base freq × tier harmonic (same as catalog_bridge::derive_item_resonance)
        let base_freq: u32 = match self.material.to_uppercase().as_str() {
            "IRON" | "STEEL" => 440,
            "STONE" | "LIMESTONE" | "GRANITE" => 220,
            "OAK" | "WOOD" | "BONE" | "ASH" => 330,
            _ => 440,
        };
        let resonance_hz = base_freq * (1 + self.tier as u32) / 2;
        CatalogPayload {
            id: self.id.clone(),
            name: self.name.clone(),
            slot: self.slot as u8,
            tier: self.tier,
            material_tag: self.material.clone(),
            socket_count: self.sockets,
            base_mass: mass,
            resonance_hz,
            tags: self.tags.iter().map(|s| s.to_string()).collect(),
        }
    }
}
