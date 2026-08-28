//! `mirror` — the deveraux.dev publish engine, folded into
//! `13forge-studio mirror publish <root> <site_url> [glyph_file]`.
//!
//! Reads captured glyphs from `<root>/inbox/*.glyph.json`, signs each into a
//! provenance seal, writes `<root>/public/entries/<seal>.svg`, folds them into the
//! feed manifest, and regenerates `public/index.html` + `public/rss.xml`.
//! Idempotent: a glyph whose seal is already published is skipped. Pure `std::fs`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{glyph_to_svg, render_index, render_rss, seal, Entry, GlyphDto, Manifest};

const SITE_TITLE: &str = "deveraux · marks";

/// `13forge-studio mirror publish <root> <site_url> [glyph_file]`. Returns exit code.
pub fn run() -> i32 {
    let args: Vec<String> = std::env::args().collect();
    // argv under the umbrella: [exe, "mirror", "publish", <root>, <site_url>, [glyph_file]]
    if args.len() < 5 || args[2] != "publish" {
        eprintln!("usage: 13forge-studio mirror publish <root> <site_url> [glyph_file]");
        return 2;
    }
    let root = PathBuf::from(&args[3]);
    let site_url = &args[4];
    let only_file = args.get(5).map(PathBuf::from);

    match publish(&root, site_url, only_file.as_deref()) {
        Ok(n) => {
            println!(
                "[forge-calligraphy] published {n} new mark(s); feed regenerated under {}/public",
                root.display()
            );
            0
        }
        Err(e) => {
            eprintln!("[forge-calligraphy] FAULT: {e}"); // Signal Law: see it = own it
            1
        }
    }
}

fn publish(root: &Path, site_url: &str, only_file: Option<&Path>) -> Result<usize, String> {
    let inbox = root.join("inbox");
    let public = root.join("public");
    let entries_dir = public.join("entries");
    let processed = root.join("processed");
    let manifest_path = public.join("entries.json");

    for d in [&inbox, &entries_dir, &processed] {
        fs::create_dir_all(d).map_err(|e| format!("mkdir {}: {e}", d.display()))?;
    }

    let mut manifest = load_manifest(&manifest_path)?;
    let now = unix_now();

    let glyph_files = match only_file {
        Some(f) => vec![if f.is_absolute() { f.to_path_buf() } else { root.join(f) }],
        None => list_glyph_files(&inbox)?,
    };

    let mut new_count = 0usize;
    for gf in glyph_files {
        let raw = fs::read_to_string(&gf).map_err(|e| format!("read {}: {e}", gf.display()))?;
        let glyph: GlyphDto =
            serde_json::from_str(&raw).map_err(|e| format!("parse {}: {e}", gf.display()))?;

        let s = seal(&glyph);
        if manifest.contains(&s.id) {
            // Already published — idempotent. Sweep the inbox file aside and move on.
            sweep(&gf, &processed);
            continue;
        }

        let svg = glyph_to_svg(&glyph);
        let svg_path = entries_dir.join(format!("{}.svg", s.id));
        fs::write(&svg_path, svg).map_err(|e| format!("write {}: {e}", svg_path.display()))?;

        manifest.push_front(Entry {
            id: s.id.clone(),
            grid_hash: s.grid_hash,
            ts_unix: now,
            title: glyph.title.clone(),
        });
        new_count += 1;
        sweep(&gf, &processed);
        println!("  + {}  (grid_hash {:#018x})", s.id, s.grid_hash);
    }

    // Regenerate the whole feed from the manifest — robust + idempotent.
    let index = render_index(&manifest, SITE_TITLE);
    let rss = render_rss(&manifest, SITE_TITLE, site_url);
    fs::write(public.join("index.html"), index).map_err(|e| format!("write index.html: {e}"))?;
    fs::write(public.join("rss.xml"), rss).map_err(|e| format!("write rss.xml: {e}"))?;
    let manifest_json =
        serde_json::to_string_pretty(&manifest).map_err(|e| format!("encode manifest: {e}"))?;
    fs::write(&manifest_path, manifest_json)
        .map_err(|e| format!("write {}: {e}", manifest_path.display()))?;

    Ok(new_count)
}

fn load_manifest(path: &Path) -> Result<Manifest, String> {
    match fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).map_err(|e| format!("parse manifest {}: {e}", path.display())),
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Manifest::default()),
        Err(e) => Err(format!("read manifest {}: {e}", path.display())),
    }
}

fn list_glyph_files(inbox: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    for entry in fs::read_dir(inbox).map_err(|e| format!("read_dir {}: {e}", inbox.display()))? {
        let entry = entry.map_err(|e| format!("dir entry: {e}"))?;
        let p = entry.path();
        if p.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(".glyph.json"))
        {
            out.push(p);
        }
    }
    out.sort(); // deterministic order
    Ok(out)
}

/// Move a processed inbox file aside so it isn't reprocessed. Janitor: never halt.
fn sweep(file: &Path, processed: &Path) {
    if let Some(name) = file.file_name() {
        let dest = processed.join(name);
        if fs::rename(file, &dest).is_err() {
            let _ = fs::copy(file, &dest).and_then(|_| fs::remove_file(file));
        }
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
