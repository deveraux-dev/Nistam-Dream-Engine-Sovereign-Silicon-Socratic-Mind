//! Build script: ensures `gifts_for_brit.html` exists before `include_str!` compiles it in.

use std::fs;
use std::path::PathBuf;

fn main() {
    let cargo_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let src_dir = PathBuf::from(cargo_dir).join("src");
    let html_path = src_dir.join("gifts_for_brit.html");

    // Create gifts_for_brit.html if it doesn't exist
    // This will be populated by the module's render_page() function at runtime
    // For now, create a placeholder that can be updated by the test
    if !html_path.exists() {
        let placeholder = "<title>100 Gifts for Brit</title><p>Placeholder - will be generated</p>";
        fs::write(&html_path, placeholder)
            .expect("Failed to write gifts_for_brit.html placeholder");
    }

    println!("cargo:rerun-if-changed=src/gifts_for_brit.rs");
    println!("cargo:rerun-if-changed=src/gifts_for_brit.css");
}
