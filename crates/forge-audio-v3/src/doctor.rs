//! `forge audio doctor` — audio-engineer skill's deterministic half (fold 07-21).
//! Suites-present gate + RT-lock sniff + dormant-dep quarry audit. Static, load-time.

use std::path::Path;

/// Non-optional `[dependencies]` keys from a Cargo.toml, as crate idents.
pub fn dep_idents(toml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_deps = false;
    for line in toml.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_deps = t == "[dependencies]";
            continue;
        }
        if !in_deps || t.is_empty() || t.starts_with('#') {
            continue;
        }
        if t.contains("optional = true") {
            continue; // feature-gated: dormancy-by-design, not a quarry
        }
        if let Some(name) = t.split('=').next() {
            let name = name.trim().trim_matches('"');
            if !name.is_empty() && name != "path" && name != "version" && name != "features" {
                out.push(name.replace('-', "_"));
            }
        }
    }
    out
}

/// DORMANT deps (skill Phase 2): declared, zero call sites in any source.
pub fn dormant_deps(deps: &[String], sources: &[(String, String)]) -> Vec<String> {
    deps.iter()
        .filter(|d| {
            let path_use = format!("{d}::");
            let use_decl = format!("use {d}");
            let extern_decl = format!("extern crate {d}");
            !sources.iter().any(|(_, src)| {
                src.contains(&path_use) || src.contains(&use_decl) || src.contains(&extern_decl)
            })
        })
        .cloned()
        .collect()
}

/// Sound-Gate sniff: blocking `.lock()` on RT-adjacent files (realtime/telemetry/bus).
/// `try_lock` is the lawful idiom and never flags; comment lines never flag.
pub fn rt_lock_hits(sources: &[(String, String)]) -> Vec<(String, usize, String)> {
    let mut hits = Vec::new();
    for (path, src) in sources {
        let p = path.replace('\\', "/");
        if !(p.contains("realtime") || p.contains("telemetry") || p.contains("/bus/")) {
            continue;
        }
        for (i, l) in src.lines().enumerate() {
            let t = l.trim_start();
            if t.starts_with("//") {
                continue;
            }
            if l.contains(".lock()") && !l.contains("try_lock") {
                hits.push((path.clone(), i + 1, l.trim().chars().take(80).collect()));
            }
        }
    }
    hits
}

/// `forge audio doctor [<crate-dir>]` — static audit of the audio organ.
pub fn run_doctor(args: Vec<String>) -> Result<(), String> {
    let dir = args
        .into_iter()
        .next()
        .unwrap_or_else(|| {
            let rel = "crates/forge-audio";
            if Path::new(rel).exists() { rel.to_string() } else { "F:/NewRepo/crates/forge-audio".to_string() }
        });
    let root = Path::new(&dir);
    if !root.join("Cargo.toml").exists() {
        return Err(format!("[doctor] no Cargo.toml under {dir}"));
    }

    // 1. Ears-proxy suites must exist (soundcheck Phase 4 anchors).
    let mut missing = Vec::new();
    for suite in ["tests/arena_test.rs", "tests/audio_quality.rs"] {
        let ok = root.join(suite).exists();
        println!("[doctor] suite {suite}: {}", if ok { "present" } else { "MISSING" });
        if !ok {
            missing.push(suite);
        }
    }

    // 2. Load sources once for both static sniffs.
    let toml = std::fs::read_to_string(root.join("Cargo.toml"))
        .map_err(|e| format!("[doctor] read Cargo.toml: {e}"))?;
    let mut sources: Vec<(String, String)> = Vec::new();
    collect_rs(&root.join("src"), &mut sources);

    // 3. RT-lock sniff (Sound Gate).
    let locks = rt_lock_hits(&sources);
    for (f, n, ex) in &locks {
        println!("[doctor] RT-LOCK {f}:{n} | {ex}");
    }

    // 4. Dormant-dep quarry audit (Phase 2 — flag LOUD, HITL rules wire-or-remove).
    let deps = dep_idents(&toml);
    let dormant = dormant_deps(&deps, &sources);
    for d in &dormant {
        println!("[doctor] DORMANT dep `{d}` — zero call sites (wire-or-remove = Sean)");
    }

    println!(
        "[doctor] {} src files · {} deps checked · {} dormant · {} RT-lock hit(s)",
        sources.len(),
        deps.len(),
        dormant.len(),
        locks.len()
    );
    println!("[doctor] arena: cargo test -p forge-audio --test arena_test · quality: --test audio_quality");
    println!("[doctor] static-only, UNHEARD — ear verification = Sean.");
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("[doctor] ears-proxy suite(s) missing: {}", missing.join(", ")))
    }
}

fn collect_rs(dir: &Path, out: &mut Vec<(String, String)>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.filter_map(Result::ok) {
        let p = e.path();
        if p.is_dir() {
            collect_rs(&p, out);
        } else if p.extension().map(|x| x == "rs").unwrap_or(false) {
            if let Ok(src) = std::fs::read_to_string(&p) {
                out.push((p.display().to_string(), src));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dep_idents_skip_optional_and_comments() {
        let toml = "[dependencies]\nlog = \"0.4\"\nforge-core = { path = \"x\" }\nhidapi = { version = \"2\", optional = true }\n# ghost = \"1\"\n[dev-dependencies]\ntempfile = \"3\"\n";
        assert_eq!(dep_idents(toml), vec!["log", "forge_core"]);
    }

    #[test]
    fn dormant_detected_and_used_cleared() {
        let deps = vec!["hound".to_string(), "rtrb".to_string()];
        let srcs = vec![("src/a.rs".to_string(), "let w = hound::WavWriter::new();".to_string())];
        assert_eq!(dormant_deps(&deps, &srcs), vec!["rtrb"]);
    }

    #[test]
    fn lock_sniff_rt_files_only_try_lock_lawful() {
        let srcs = vec![
            ("src/realtime.rs".to_string(), "m.lock();\nm.try_lock();\n// m.lock()\n".to_string()),
            ("src/ingest.rs".to_string(), "m.lock();\n".to_string()),
        ];
        let hits = rt_lock_hits(&srcs);
        assert_eq!(hits.len(), 1);
        assert_eq!((hits[0].0.as_str(), hits[0].1), ("src/realtime.rs", 1));
    }
}
