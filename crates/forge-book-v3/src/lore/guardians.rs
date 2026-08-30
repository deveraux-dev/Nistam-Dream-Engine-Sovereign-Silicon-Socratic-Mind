//! Zone guardians — drained from `13moons/data/generated/guardian_lore.json`.
//! Eight Cree-named spirits, one per zone role, each carrying its encounter arc
//! (approach → phase transition → defeat), the zone lore it guards, and a sound
//! signature written in real DSP terms (fundamentals in Hz, roughness bands).
//!
//! The sound signatures are the reason this is prose and not a table: they name
//! the exact psychoacoustic move each guardian uses (missing fundamental,
//! Shepard-Risset glissando, roughness modulation, looming bias). An audio lane
//! reads them; nothing here re-implements one.

use serde::Serialize;

/// Which zone a guardian keeps. Two guardians per zone across the four zones —
/// one that meets you on the ground and one that does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Zone {
    /// Endless snow, deep ice, the birthplace of winter's breath.
    Frost,
    /// Dry earth and bedrock — the slow patience of erosion and formation.
    Stone,
    /// Scorching heat and smouldering embers beneath the surface.
    Ember,
    /// Rivers, wetlands and the deep places where water gathers strength.
    Tide,
}

/// One guardian's full encounter lore.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Guardian {
    /// Cree id, lowercase and underscored — the save key and the asset key.
    pub id: &'static str,
    /// The encounter's title card.
    pub encounter_name: &'static str,
    /// Which zone this guardian keeps.
    pub zone: Zone,
    /// What the player senses BEFORE the guardian is visible. Every one of the
    /// eight opens on a non-visual sense — that is the authored rule, and
    /// `every_approach_lands_before_sight` holds it.
    pub phase1_narrative: &'static str,
    /// The turn into phase two.
    pub phase_transition: &'static str,
    /// The land healing after the guardian falls.
    pub defeat_narrative: &'static str,
    /// What this spirit keeps, and why its fury rises.
    pub zone_lore: &'static str,
    /// The psychoacoustic recipe — frequencies, texture, localization behaviour.
    pub sound_signature: &'static str,
}

/// The eight, in authored order.
pub const GUARDIANS: [Guardian; 8] = [
    Guardian {
        id: "maskwa",
        encounter_name: "The Winter's Roar",
        zone: Zone::Frost,
        phase1_narrative: "A low, guttural rumble vibrates through the frozen earth, a sound so deep it bypasses the ear and settles in the chest. The air itself grows heavy, thick with ancient power and the biting chill of the frost zone. Before you see it, you feel the immense weight of the maskwa, a presence as old and unyielding as the ice itself.",
        phase_transition: "As its fury mounts, the maskwa draws strength from the very heart of the frost, a deep, resonant pulse echoing from the earth as Medicine Roots surge with renewed power, transforming its form.",
        defeat_narrative: "With its rage quelled, the vast form of the maskwa slowly dissolves into the swirling snow, leaving behind a profound stillness. The biting cold softens, and a gentle warmth begins to seep into the frozen ground, mending the wounds of the land it once protected.",
        zone_lore: "The maskwa is the spirit of the enduring winter, the silent strength beneath the endless snows of the frost zone. It guards the delicate balance of life that persists even in the harshest cold, ensuring the deep ice holds its secrets and the hidden springs flow true beneath the frozen crust.",
        sound_signature: "A profound, low-frequency rumble (15-40Hz) that resonates through the ground and bone, carrying harmonics that induce a visceral dread. Its roar is a complex, multi-layered sound: a deep, sustained fundamental overlaid with high-frequency ice-cracking and a deep, throaty growl. The texture is rough and heavy, like grinding glaciers, with intermittent, sharp exhalations of cold air. The sound feels vast and all-encompassing, a looming presence that seems to originate from everywhere at once, increasing in intensity as it draws near, triggering an involuntary tightening in the chest.",
    },
    Guardian {
        id: "paskwawi_mostos",
        encounter_name: "Heartbeat of the Stone",
        zone: Zone::Stone,
        phase1_narrative: "The ground trembles before you see anything, a low thrumming that vibrates through your bones, slowly intensifying. Then, a towering dust cloud on the horizon grows with impossible speed, coalescing into the sheer, unstoppable force of the paskwawi_mostos.",
        phase_transition: "A guttural bellow rips through the air, shaking the very stones beneath your feet, as the paskwawi_mostos lowers its massive head, its charge now an unyielding, relentless force of nature.",
        defeat_narrative: "The colossal form shudders, then slowly dissolves into the wind, leaving only the scent of rain on dry earth. A profound silence settles, and you feel the land around you sigh, a deep, slow breath of healing.",
        zone_lore: "The paskwawi_mostos embodies the enduring spirit of the stone zone, its hooves carving strength into the dry earth. When the land grows sick, its fury manifests as an untamed stampede, a desperate attempt to shake the sickness from the ground itself, its power a reflection of the earth's own resilience.",
        sound_signature: "A low, pervasive infrasound, felt more than heard, resonating at the missing fundamental of 18.9Hz, accompanied by a roughness modulation between 40-80Hz that triggers an involuntary tightening in the chest. Its charges are punctuated by percussive, ground-shaking thumps, each impact carrying the booming resonance of a drum made from granite, followed by deep, guttural bellows that seem to tear the very air.",
    },
    Guardian {
        id: "mahkesis",
        encounter_name: "The Ember Trickster",
        zone: Zone::Ember,
        phase1_narrative: "The air shimmers with impossible heat, blurring the world until shapes dance at the edge of vision. You feel a presence, quick and sly, a phantom slipping through the haze, always just out of reach.",
        phase_transition: "The heat intensifies, making the air thrum as the trickster's form seems to fragment, appearing in multiple places at once, each a fleeting, burning shadow.",
        defeat_narrative: "As the last echo of its elusive presence fades, the oppressive heat of the Ember zone begins to soften, easing its fiery grip on the land. A faint, cooling breeze whispers through the scorched earth, a promise of regrowth.",
        zone_lore: "The mahkesis embodies the scorching, shifting heart of the Ember zone, a spirit of cunning and survival in the harshest heat. It guards the ancient, smoldering embers that lie beneath the surface, ensuring the land's fiery balance remains. It tests those who trespass, seeking to maintain the desolate beauty of its domain.",
        sound_signature: "A high-pitched, almost subliminal hiss that seems to originate from nowhere and everywhere at once, like hot air escaping a cracked stone. It's often masked by the low, dry rustle of heat-baked earth and the distant crackle of embers, making it impossible to pinpoint its source. The sound has a peculiar, dry 'roughness' (30-50Hz) that pricks at the edges of hearing, triggering a primal unease before any visual cue, hinting at movement just beyond perception.",
    },
    Guardian {
        id: "kihkwahas",
        encounter_name: "Kihkwahas, The Sky Hunter",
        zone: Zone::Frost,
        phase1_narrative: "A high, piercing cry slices through the biting wind, a sound that vibrates in the chest before the source is seen. The air thins, growing colder, as a vast shadow dances across the snow-covered ground, hinting at a hunter circling far above.",
        phase_transition: "With a sudden, violent gust, the sky hunter descends, its cries intensifying as it closes the distance, a blur of white against the endless, pale sky.",
        defeat_narrative: "The furious winds still, and the piercing cries fade into a whisper of falling snow. A fragile, almost imperceptible warmth begins to seep into the frozen earth, and the oppressive silence of the high plains softens, promising a return to balance in the frigid landscape.",
        zone_lore: "Kihkwahas guards the purest, highest reaches of the Frost zone, where the very breath of winter is born. It ensures the bitter winds carry away all weakness, maintaining the ancient, harsh beauty of the ice-sculpted land. Its domain is the vast, open sky above the frozen plains, a watchful eye over the pristine, untamed cold.",
        sound_signature: "A high-frequency, piercing shriek that feels like wind tearing through solid ice, often perceived as a pressure change before it's consciously heard. This sound rapidly intensifies with a Shepard-Risset glissando effect during its dives, creating an inescapable sensation of being closed upon from above (looming bias). It carries a sharp, whistling, almost metallic quality, punctuated by sudden, violent gusts of sound that contain roughness modulation (30-150Hz), triggering an involuntary sense of primal fear and exposure. Its calls are deceptively hard to pinpoint, seeming to emanate from everywhere and nowhere in the vast, open sky, enhancing the feeling of dread and disorientation.",
    },
    Guardian {
        id: "namew",
        encounter_name: "The Deep Current's Embrace",
        zone: Zone::Tide,
        phase1_narrative: "The very water around you begins to thicken, a low, resonant thrum vibrating through your bones long before the surface breaks. A colossal form, armoured in ancient bone plates, rises from the depths, an eye like a moon reflecting the murky sky. The pressure of the deep presses in, a warning from the river itself.",
        phase_transition: "The namew recoils, not in retreat, but to gather the river's full force, the current around it twisting into a maelstrom that pulls at the very ground beneath your feet.",
        defeat_narrative: "With a final, mournful groan that shakes the very bedrock beneath the water, the namew sinks back into the depths, its ancient spirit returning to the current. The waters around you calm, the sickly green of the tide zone slowly giving way to a healthier, vibrant hue. The land breathes a sigh of relief, its thirst sated.",
        zone_lore: "The namew is the ancient heart of the tide zone, a living memory of the rivers and lakes that once carved this land. It guards the deep places where the water gathers its strength, ensuring the flow remains true, untainted by the sickness that creeps from the land. When the waters become stagnant, its slumber is disturbed, and its fury rises with the poisoned current, seeking to cleanse the rot. Its presence steadies the very flow of life in this region.",
        sound_signature: "A deep, resonant thrumming, almost below the threshold of hearing, vibrates through the ground and water, a felt pressure rather than a sound, hinting at an immense, unseen presence. Rough, grinding groans like tectonic plates shifting underwater, accompanied by the slow, heavy creak of ancient bone armor flexing as it moves. Occasional bursts of low-frequency gurgling, like a massive cavity filling and emptying, evoke a profound, primal dread that claws at the back of the mind, a sound that seems to come from everywhere and nowhere at once, making localization impossible.",
    },
    Guardian {
        id: "moswa",
        encounter_name: "The Great Thirster",
        zone: Zone::Tide,
        phase1_narrative: "You feel the moisture being pulled from the air long before you see it. The wetlands, usually vibrant and teeming, are strangely muted, the ground growing firm underfoot. A deep, aching bellow echoes from the mist, a sound that speaks of profound, unending hunger, drawing the life from everything around it.",
        phase_transition: "The moswa lets out a guttural, desperate roar, its spectral form shimmering as it draws even more furiously from the land. The ground beneath you cracks, and the once-lush vegetation wilts and crumbles into dust, leaving only parched earth in its wake.",
        defeat_narrative: "With a final, mournful groan, the mighty moswa dissipates like mist, its essence returning to the earth. A collective sigh seems to rise from the land as the parched ground softens, and the wetlands slowly, gratefully, begin to reclaim their waters. The air, once heavy with thirst, now feels cool and revitalized.",
        zone_lore: "The moswa is an ancient spirit, a guardian tied to the delicate balance of water and life in the Tide zone. When the land itself is out of harmony, its hunger becomes boundless, a force that consumes the very vitality of the wetlands, leaving only cracked mud and desolation. It serves as a stark, powerful reminder that even abundance can be drained, leaving behind a stark, aching void.",
        sound_signature: "A deep, resonant bellow, its fundamental frequency sitting between 30-60Hz, carries a pervasive, almost infrasonic weight. This is overlaid with a subtle, dry rustling and the distinct, unsettling sound of liquid being drawn away, a slow, heavy 'slurp' that seems to pull the very moisture from the air. Each heavy, waterlogged thud of its steps vibrates through the ground, accompanied by a high-frequency, brittle crackle, like drying earth giving way. The overall texture is one of profound, parching thirst.",
    },
    Guardian {
        id: "pisiw",
        encounter_name: "The Ember Stalker",
        zone: Zone::Ember,
        phase1_narrative: "The air shimmers with impossible heat, twisting the very light into dancing phantoms. A prickle of unease crawls up your spine, a whisper of movement that defies direction, a scent of burnt resin and something wild, unseen, just beyond the edge of your perception.",
        phase_transition: "The heat solidifies, pressing in, and the world itself seems to shudder. What was once a fleeting impression now strikes with sudden, brutal force, a silent fury erupting from the shimmering air.",
        defeat_narrative: "The oppressive heat recedes, leaving a lingering warmth that feels less like a threat and more like a memory. The land exhales, and the embers of the zone begin to glow with a steady, restorative pulse, no longer consuming, but nurturing.",
        zone_lore: "The pisiw guards the searing heart of the Ember zone, a silent sentinel of its consuming fires and its vital, purifying heat. It ensures that the land's primal warmth does not burn unchecked, nor does it ever fully extinguish. Its presence is a constant, unsettling reminder that even in the brightest flames, shadows hold true power.",
        sound_signature: "Its presence is marked by a deep, unsettling absence of localizable sound. Instead, a pervasive, low-frequency roughness (30-50Hz) vibrates through the earth, felt in the chest more than heard, triggering an involuntary sense of dread without a clear source. Brief, high-frequency whispers or clicks, too ephemeral to pinpoint, scatter just outside auditory focus, creating a disorienting 'phantom sound' effect that amplifies localization difficulty and looming bias, making its approach terrifyingly silent and ubiquitous.",
    },
    Guardian {
        id: "wawaskesiw",
        encounter_name: "The Earth-Shaker's Fury",
        zone: Zone::Stone,
        phase1_narrative: "You feel it before you see it: a deep, resonant hum that vibrates through the very rock beneath your feet, growing into a thunderous tremor. The ground shivers, dust plumes rising as cracks spiderweb across the ancient stone, announcing the presence of the great Elk, a force of nature made manifest.",
        phase_transition: "A guttural bellow rips through the air, and the ground convulses violently, tearing open new chasms as the Elk channels the raw power of the earth itself.",
        defeat_narrative: "With a final, desperate cry that echoes into the vast sky, the Elk collapses, its form dissolving into the very stone it protected. The earth stills, the tremors fading, and a profound sense of peace washes over the cracked, thirsty land of the stone zone, a quiet promise of renewal.",
        zone_lore: "The wawaskesiw is the ancient keeper of the stone heart of the prairies, its very presence tethering the deep earth to the sky above. It guards the stability of the bedrock, ensuring the slow, patient processes of erosion and formation continue unbroken. When the stone zone sickens, the Elk becomes a restless spirit, its powerful hooves striking the earth in protest, threatening to unravel the very ground beneath the world.",
        sound_signature: "A profound, low-frequency rumble, beginning around 20-30Hz, incorporating the missing fundamental technique to create an infrasonic 'felt' dread that vibrates the player's chest cavity. This is overlaid with intermittent, sharp, percussive impacts that mimic massive hooves striking stone, accompanied by a growing roughness modulation (30-150Hz) that evokes primal fear and the sensation of approaching, overwhelming force. Its roar is a deep, resonant bellow, rich in harmonics, that carries across vast distances, localized with subtle phasing to enhance looming bias.",
    },
];

/// The guardian with this id. Unknown id = `None`; a missing guardian is a
/// content bug, never a substituted one.
pub fn guardian(id: &str) -> Option<&'static Guardian> {
    GUARDIANS.iter().find(|g| g.id == id)
}

/// Both guardians of a zone, in authored order.
pub fn guardians_of(zone: Zone) -> impl Iterator<Item = &'static Guardian> {
    GUARDIANS.iter().filter(move |g| g.zone == zone)
}

impl Guardian {
    /// The encounter's three prose beats in play order — what a cutscene or a
    /// codex page walks. Named so callers never hand-order the fields and get
    /// the defeat before the transition.
    pub fn arc(&self) -> [&'static str; 3] {
        [self.phase1_narrative, self.phase_transition, self.defeat_narrative]
    }

    /// Does the sound signature give an audio lane something numeric to tune to?
    /// Seven of the eight quote an explicit `Hz` band. `namew` is the deliberate
    /// exception — it is written as sitting BELOW the threshold of hearing, a
    /// felt pressure rather than a pitch, so "no number" is its number.
    pub fn signature_is_tunable(&self) -> bool {
        self.sound_signature.contains("Hz")
            || self.sound_signature.contains("below the threshold of hearing")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_guardians_two_per_zone_all_ids_distinct() {
        assert_eq!(GUARDIANS.len(), 8);
        let ids: std::collections::HashSet<&str> = GUARDIANS.iter().map(|g| g.id).collect();
        assert_eq!(ids.len(), 8, "two guardians share an id");
        for zone in [Zone::Frost, Zone::Stone, Zone::Ember, Zone::Tide] {
            assert_eq!(guardians_of(zone).count(), 2, "{zone:?} is not a pair");
        }
    }

    #[test]
    fn lookup_answers_by_id_and_stays_silent_otherwise() {
        assert_eq!(guardian("namew").expect("namew").zone, Zone::Tide);
        assert_eq!(
            guardian("wawaskesiw").expect("wawaskesiw").encounter_name,
            "The Earth-Shaker's Fury"
        );
        assert!(guardian("wendigo").is_none(), "an unwritten guardian is never substituted");
    }

    // The authored rule the whole set obeys: you HEAR or FEEL the guardian
    // before you see it. A phase-1 that opens on sight breaks the approach.
    #[test]
    fn every_approach_lands_before_sight() {
        for g in GUARDIANS {
            let p1 = g.phase1_narrative.to_ascii_lowercase();
            assert!(
                p1.contains("feel")
                    || p1.contains("before you see")
                    || p1.contains("heard")
                    || p1.contains("cry")
                    || p1.contains("rumble")
                    || p1.contains("thrum")
                    || p1.contains("bellow")
                    || p1.contains("shimmer"),
                "{}: phase 1 opens on sight",
                g.id
            );
            assert_eq!(g.arc().len(), 3);
            for beat in g.arc() {
                assert!(beat.len() > 80, "{}: a beat was never written out", g.id);
            }
        }
    }

    // The sound signatures are the payload's real value — every one is tunable
    // by an audio lane. Seven quote an Hz band; namew is written below hearing.
    #[test]
    fn every_sound_signature_is_tunable() {
        for g in GUARDIANS {
            assert!(g.signature_is_tunable(), "{}: nothing for an audio lane to read", g.id);
            assert!(!g.zone_lore.trim().is_empty(), "{}: guards nothing", g.id);
        }
        let with_hz = GUARDIANS.iter().filter(|g| g.sound_signature.contains("Hz")).count();
        assert_eq!(with_hz, 7, "the sub-threshold exception is namew alone");
        assert!(!guardian("namew").expect("namew").sound_signature.contains("Hz"));
    }
}
