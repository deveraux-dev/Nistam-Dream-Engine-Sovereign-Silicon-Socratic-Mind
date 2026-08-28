//! PCM decode cache — SHA-256 file signature → raw f32 on disk.
//! First load decodes via symphonia and caches. Subsequent loads read raw PCM (~150ms vs 4-5s).

use sha2::{Sha256, Digest};
use std::path::PathBuf;
use std::io::Write;

fn cache_dir() -> PathBuf {
    // Store cache next to the app data
    let base = std::env::var("LOCALAPPDATA")
        .unwrap_or_else(|_| std::env::var("HOME").unwrap_or_else(|_| ".".into()));
    PathBuf::from(base).join("DeadDrop").join("pcm_cache")
}

fn cache_key(path: &str) -> Result<String, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("cache key: {}", e))?;
    let mut hasher = Sha256::new();
    // Hash path + file size — instant, no file I/O beyond stat
    hasher.update(path.as_bytes());
    hasher.update(meta.len().to_le_bytes());
    let hash = hasher.finalize();
    Ok(format!("{:x}", hash)[..16].to_string())
}

fn pcm_path(key: &str) -> PathBuf { cache_dir().join(format!("{}.pcm", key)) }
fn meta_path(key: &str) -> PathBuf { cache_dir().join(format!("{}.json", key)) }

/// Try to load from cache. Returns None on miss.
fn load_cached(key: &str) -> Option<crate::dsp::AudioBuffer> {
    let meta_p = meta_path(key);
    let pcm_p = pcm_path(key);
    if !meta_p.exists() || !pcm_p.exists() { return None; }

    let meta_str = std::fs::read_to_string(&meta_p).ok()?;
    let meta: serde_json::Value = serde_json::from_str(&meta_str).ok()?;
    let version = meta["version"].as_u64().unwrap_or(1);
    if version < 2 { return None; } // v1 was interleaved, incompatible
    let sample_rate = meta["sample_rate"].as_u64()? as u32;
    let channels = meta["channels"].as_u64()? as usize;

    let raw = std::fs::read(&pcm_p).ok()?;
    if raw.len() % 4 != 0 { return None; }
    let total_samples = raw.len() / 4;
    let per_channel = total_samples / channels;

    // Direct transmute: bytes → f32 slice (zero per-sample conversion)
    // Safety: raw is aligned to 1, f32 requires 4-byte alignment, so we copy via from_le
    // but do it per-channel in one contiguous block instead of scattering
    let mut samples = Vec::with_capacity(channels);
    for ch in 0..channels {
        let byte_offset = ch * per_channel * 4;
        let byte_end = byte_offset + per_channel * 4;
        if byte_end > raw.len() { return None; }
        let channel_data: Vec<f32> = raw[byte_offset..byte_end]
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        samples.push(channel_data);
    }

    Some(crate::dsp::AudioBuffer { samples, sample_rate })
}

/// Save decoded audio to cache.
fn save_cached(key: &str, buf: &crate::dsp::AudioBuffer) -> Result<(), String> {
    let dir = cache_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("cache mkdir: {}", e))?;

    // Write per-channel contiguous raw f32 (ch0 then ch1)
    let channels = buf.channels();
    let len = buf.len();
    let mut raw = Vec::with_capacity(channels * len * 4);
    for ch in 0..channels {
        for &s in &buf.samples[ch] {
            raw.extend_from_slice(&s.to_le_bytes());
        }
    }
    let mut f = std::fs::File::create(pcm_path(key)).map_err(|e| format!("cache write: {}", e))?;
    f.write_all(&raw).map_err(|e| format!("cache write: {}", e))?;

    // Write metadata
    let meta = serde_json::json!({
        "sample_rate": buf.sample_rate,
        "channels": channels,
        "duration_secs": buf.duration_secs(),
        "version": 2,
    });
    std::fs::write(meta_path(key), meta.to_string()).map_err(|e| format!("meta write: {}", e))?;

    Ok(())
}

/// Max cache size in bytes (2 GB). Oldest files evicted when exceeded.
const MAX_CACHE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// Evict oldest .pcm files until total cache size is under the limit.
fn evict_if_needed() {
    let dir = cache_dir();
    let entries: Vec<_> = match std::fs::read_dir(&dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
        Err(_) => return,
    };
    // Collect .pcm files with size and modified time.
    let mut files: Vec<(PathBuf, u64, std::time::SystemTime)> = entries.iter()
        .filter(|e| e.path().extension().is_some_and(|x| x == "pcm"))
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            Some((e.path(), meta.len(), meta.modified().ok()?))
        })
        .collect();
    let total: u64 = files.iter().map(|(_, s, _)| s).sum();
    if total <= MAX_CACHE_BYTES { return; }
    // Sort oldest first, evict until under limit.
    files.sort_by_key(|(_, _, t)| *t);
    let mut freed = 0u64;
    let target = total - MAX_CACHE_BYTES;
    for (path, size, _) in &files {
        if freed >= target { break; }
        let meta_p = path.with_extension("json");
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(meta_p);
        freed += size;
        eprintln!("[Cache] evicted {} ({:.1}MB)", path.display(), *size as f64 / 1_048_576.0);
    }
}

/// Load audio with cache. Cache hit = ~150ms, miss = decode + cache.
pub fn get_or_load(path: &str) -> Result<crate::dsp::AudioBuffer, String> {
    let key = cache_key(path)?;

    // Cache HIT
    if let Some(buf) = load_cached(&key) {
        eprintln!("[Cache] HIT {} → {:.1}s, {} ch, {} Hz",
            &key[..8], buf.duration_secs(), buf.channels(), buf.sample_rate);
        return Ok(buf);
    }

    // Cache MISS — decode and save
    eprintln!("[Cache] MISS {} — decoding", &key[..8]);
    let buf = crate::dsp::load_audio(path)?;
    if let Err(e) = save_cached(&key, &buf) {
        eprintln!("[Cache] save failed: {} (continuing without cache)", e);
    } else {
        eprintln!("[Cache] saved {:.1}MB", (buf.channels() * buf.len() * 4) as f64 / 1_048_576.0);
        evict_if_needed();
    }
    Ok(buf)
}
