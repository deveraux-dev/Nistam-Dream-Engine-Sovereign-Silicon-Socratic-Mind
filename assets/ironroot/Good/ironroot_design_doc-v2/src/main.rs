pub mod tone_and_atmosphere;
pub mod bard_path_mechanics;
pub mod world_encounters;
pub mod grand_ending;
pub mod design_guidelines;

fn main() {
    println!("Ironroot Design Document: The Plain Song Route loaded.");
    println!("Core Concept: {}", tone_and_atmosphere::get_tone_and_atmosphere().core_concept);
}
