//! Sphere Pixelizer chapter — the star index primitive, documented from proven code.
//! Source: forge-ml/src/sphere_index.rs (7 tests green) + _plans/star-index-primitives-2026-07-18.md.

use crate::atlas::AtlasSection;
use crate::block::Block;
use crate::chapter::Chapter;
use crate::page::Page;

/// One of the seven primitives the 5D star index reduces to, with its cross-lineage grounding.
struct Primitive {
    name: &'static str,
    reduces_to: &'static str,
    lineage: &'static str,
    forge: &'static str,
}

const PRIMITIVES: &[Primitive] = &[
    Primitive { name: "ORIGIN", reduces_to: "a fixed datum", lineage: "Ekakatchet Atchakos — the Cree standing-still star (Polaris)", forge: "river.idx HEAD" },
    Primitive { name: "RAY", reduces_to: "a bearing from origin", lineage: "the cross-staff / backstaff altitude angle (1500-1700s)", forge: "raycast(from,toward)" },
    Primitive { name: "DICHOTOMY", reduces_to: "recursive halving -> a DAG", lineage: "Ramus's 1500s bracket-tree; the HEALPix quadtree", forge: "Morton nest · forge-dag" },
    Primitive { name: "BASE-12", reduces_to: "12 coarse cells", lineage: "the zodiac's 12 houses = HEALPix's 12 faces", forge: "sphere_index::BASE_FACES" },
    Primitive { name: "FOLD", reduces_to: "one hash -> many lanes", lineage: "Mersenne 2^13-1 = 8191, the two-for-one split", forge: "fold_lanes -> [x,y,z,w,theta]" },
    Primitive { name: "FIVE LANES", reduces_to: "the 5-tuple", lineage: "a star row = position + magnitude + epoch + proper-motion", forge: "river.idx 5D box" },
    Primitive { name: "CATALOG", reduces_to: "the named indexed set", lineage: "Tycho -> Bayer -> Flamsteed; the Cree sky map", forge: "the 100-cell CREE codebook" },
];

/// Build the "Sphere Pixelizer" chapter: the seven primitives the 5D star index
/// reduces to, each attested across Cree sky-navigation, 1500-1700s astronomy, the
/// Ramist DAG, and Mersenne math — proven in forge-ml/src/sphere_index.rs (7 green).
pub fn sphere_index_chapter(title: impl Into<String>) -> Chapter {
    let mut ch = Chapter::new(title, AtlasSection::Custom("Sky".into()));
    ch.add_lore(
        "The stars are the answer, made of bits. A star catalogue, a Ramist bracket-tree, and a \
         cross-staff are the same seven primitives — and the forge already ran four of them. \
         Twelve faces, a Morton quadtree, a Mersenne fold: one u64 pixel names any cell of the sky.",
    );

    let mut prims = Page::new(1);
    prims.add(Block::text("The seven primitives (name — reduces to — lineage — forge organ):"));
    for p in PRIMITIVES {
        prims.add(Block::text(format!("  {} — {} — {} — {}", p.name, p.reduces_to, p.lineage, p.forge)));
    }
    ch.add_page(prims);

    let mut nums = Page::new(2);
    nums.add(Block::text("The numbers (proven — forge-ml/src/sphere_index.rs, 7 tests green):"));
    nums.add(Block::text("  order 7 -> Nside 128 -> 16384 = 2^14 pixels per face -> 12 x 16384 = 196608 total."));
    nums.add(Block::text("  16384 = a Ramist dichotomy fourteen levels deep. 8191 = 2^13-1, the Mersenne prime (13 = the Forge number)."));
    nums.add(Block::text("  fold_lanes: one FNV-1a hash, split five-for-one mod 8191, becomes the [x,y,z,w,theta] box point."));
    nums.add(Block::text("  Live caller: index_cree_codebook maps all 100 CREE cells across the 12 faces — the index indexing the alphabet."));
    ch.add_page(nums);

    ch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sphere_chapter_carries_the_seven_primitives_and_the_numbers() {
        let ch = sphere_index_chapter("Sphere Pixelizer");
        assert_eq!(ch.section, AtlasSection::Custom("Sky".into()));
        assert_eq!(PRIMITIVES.len(), 7);
        let text: String = ch
            .pages
            .iter()
            .flat_map(|p| p.blocks.iter())
            .map(|b| b.as_plain())
            .collect::<Vec<_>>()
            .join("\n");
        for needle in ["ORIGIN", "DICHOTOMY", "BASE-12", "196608", "8191", "index_cree_codebook"] {
            assert!(text.contains(needle), "sphere chapter missing '{needle}'");
        }
    }
}
