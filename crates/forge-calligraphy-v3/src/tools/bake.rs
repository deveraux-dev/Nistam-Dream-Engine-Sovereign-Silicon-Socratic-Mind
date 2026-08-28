//! `bake` — portfolio baker, folded into `13forge-studio bake <portfolio-dir>`.
//! Reads `portfolio.json` + image files from a dir, emits a self-contained
//! `bundle.html` (zero external requests). On any error: non-zero exit, never a
//! partial file (ARCH-001 Signal Law).

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::{render_gallery, GalleryError, GalleryItem, PortfolioManifest};

/// Authored form of the portfolio — references images by filename; baker fills `bytes`.
#[derive(Deserialize)]
struct PortfolioDoc {
    title: String,
    items: Vec<ItemRef>,
}

#[derive(Deserialize)]
struct ItemRef {
    caption: String,
    alt: String,
    mime: String,
    /// Filename relative to the portfolio dir (no `..` traversal allowed).
    src: String,
}

/// `13forge-studio bake <portfolio-dir>` → writes `<dir>/bundle.html`. Returns the
/// process exit code (0 ok · 1 fault · 2 usage).
pub fn run() -> i32 {
    let args: Vec<String> = std::env::args().collect();
    // argv under the umbrella: [exe, "bake", <portfolio-dir>]
    if args.len() != 3 {
        eprintln!("usage: 13forge-studio bake <portfolio-dir>");
        return 2;
    }
    let dir = PathBuf::from(&args[2]);
    match bake_dir(&dir) {
        Ok(out) => {
            println!("[bake] wrote {}", out.display());
            0
        }
        Err(e) => {
            eprintln!("[bake] FAULT: {e}");
            1
        }
    }
}

/// Reject any src that would escape the portfolio sandbox (ARCH-005 VFS Seam).
fn safe_join(base: &Path, rel: &str) -> Result<PathBuf, String> {
    if rel.contains("..") || Path::new(rel).is_absolute() {
        return Err(format!("path traversal rejected: {rel:?}"));
    }
    Ok(base.join(rel))
}

fn bake_dir(dir: &Path) -> Result<PathBuf, String> {
    let doc_path = dir.join("portfolio.json");
    let doc_bytes = fs::read(&doc_path)
        .map_err(|e| format!("cannot read {}: {e}", doc_path.display()))?;
    let doc: PortfolioDoc = serde_json::from_slice(&doc_bytes)
        .map_err(|e| format!("portfolio.json parse error: {e}"))?;

    let mut items: Vec<GalleryItem> = Vec::with_capacity(doc.items.len());
    for (i, r) in doc.items.iter().enumerate() {
        let img_path = safe_join(dir, &r.src)?;
        let bytes = fs::read(&img_path)
            .map_err(|e| format!("item {i}: cannot read {}: {e}", img_path.display()))?;
        if bytes.is_empty() {
            return Err(format!("item {i}: {} is empty (Signal Law)", img_path.display()));
        }
        items.push(GalleryItem {
            caption: r.caption.clone(),
            alt: r.alt.clone(),
            mime: r.mime.clone(),
            bytes,
        });
    }

    let manifest = PortfolioManifest { title: doc.title, items };
    // Build the full HTML in memory before touching the output file — never write partial.
    let html = render_gallery(&manifest).map_err(|e| match e {
        GalleryError::EmptyAsset(i) => format!("item {i}: empty bytes (Signal Law)"),
        GalleryError::BadMime(i, mime) => format!("item {i}: unknown MIME {mime:?} (Signal Law)"),
    })?;

    let out = dir.join("bundle.html");
    fs::write(&out, html.as_bytes()).map_err(|e| format!("cannot write {}: {e}", out.display()))?;
    Ok(out)
}
