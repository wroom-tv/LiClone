use std::collections::HashMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime};

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::cache::cache_root_paths;
use crate::discover::list_instances;
use crate::parse::flag_value;
use crate::rc::TransferItem;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservedTransfers {
    pub transferring: Vec<TransferItem>,
    pub queued: u64,
    pub uploading: u64,
    pub dirty_bytes: u64,
    pub recently_done: Vec<TransferItem>,
}

#[derive(Deserialize, Default)]
struct VfsMeta {
    #[serde(default, alias = "Dirty", alias = "dirty")]
    dirty: bool,
    #[serde(default, alias = "Size", alias = "size")]
    size: i64,
}

struct Sample {
    bytes: u64,
    at: Instant,
}

struct Tracker {
    samples: HashMap<String, Sample>,
    dirty: HashMap<String, i64>,
    done: Vec<TransferItem>,
    ready: bool,
}

fn tracker() -> &'static Mutex<Tracker> {
    static TRACK: OnceLock<Mutex<Tracker>> = OnceLock::new();
    TRACK.get_or_init(|| {
        Mutex::new(Tracker {
            samples: HashMap::new(),
            dirty: HashMap::new(),
            done: Vec::new(),
            ready: false,
        })
    })
}

fn item(
    name: String,
    size: i64,
    bytes: i64,
    speed: f64,
    phase: &str,
    src: &str,
    dst: &str,
) -> TransferItem {
    let percentage = if size > 0 {
        ((bytes as f64) / (size as f64) * 100.0).clamp(0.0, 100.0)
    } else if bytes > 0 {
        100.0
    } else {
        0.0
    };
    let eta = if speed > 1.0 && size > bytes {
        Some(((size - bytes) as f64 / speed).round() as i64)
    } else {
        None
    };
    TransferItem {
        name,
        size,
        bytes,
        percentage,
        speed,
        speed_avg: speed,
        eta,
        group: Some(phase.into()),
        src_fs: Some(src.into()),
        dst_fs: Some(dst.into()),
        phase: Some(phase.into()),
    }
}

fn is_fresh(path: &Path) -> bool {
    path.metadata()
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .map(|d| d.as_secs() < 8)
        .unwrap_or(false)
}

fn speed_for(key: &str, bytes: u64, track: &mut Tracker) -> f64 {
    let now = Instant::now();
    let speed = if let Some(prev) = track.samples.get(key) {
        let dt = now.duration_since(prev.at).as_secs_f64();
        if dt > 0.2 {
            ((bytes as f64) - (prev.bytes as f64)).max(0.0) / dt
        } else {
            0.0
        }
    } else {
        0.0
    };
    track.samples.insert(
        key.to_string(),
        Sample {
            bytes,
            at: now,
        },
    );
    speed
}

fn scan_vfs_meta(root: &Path, track: &mut Tracker, out: &mut Vec<TransferItem>) -> (u64, u64, u64) {
    let meta_root = root.join("vfsMeta");
    let data_root = root.join("vfs");
    if !meta_root.is_dir() {
        return (0, 0, 0);
    }
    let mut queued = 0u64;
    let mut uploading = 0u64;
    let mut dirty_bytes = 0u64;

    for entry in WalkDir::new(&meta_root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        let Ok(meta) = serde_json::from_str::<VfsMeta>(&text) else {
            continue;
        };
        let rel = path
            .strip_prefix(&meta_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let data_path = data_root.join(path.strip_prefix(&meta_root).unwrap_or(path));
        let on_disk = fs::metadata(&data_path).map(|m| m.len()).unwrap_or(0);
        let expected = if meta.size > 0 { meta.size } else { on_disk as i64 };

        // Dirty means the bytes are on disk and rclone has not uploaded them yet.
        // A partial cache with Dirty false is a read filling the cache — leave it out.
        if !meta.dirty {
            continue;
        }
        let key = format!("dirty:{rel}");
        let seen = track.samples.contains_key(&key);
        let spd = speed_for(&key, on_disk, track);
        dirty_bytes += on_disk;
        let local_size = expected.max(on_disk as i64);
        let still_writing = spd > 256.0 || (!seen && is_fresh(&data_path) && on_disk + 4096 < local_size as u64);
        if still_writing {
            uploading += 1;
            out.push(item(
                rel,
                local_size,
                on_disk as i64,
                spd,
                "writing",
                "disk",
                "cloud",
            ));
        } else {
            queued += 1;
            let mut t = item(
                rel,
                local_size,
                on_disk as i64,
                0.0,
                "queued",
                "disk",
                "cloud",
            );
            t.percentage = 0.0;
            t.eta = None;
            out.push(t);
        }
    }
    (queued, uploading, dirty_bytes)
}

fn log_errors(path: &Path) -> Vec<String> {
    let Ok(mut f) = fs::File::open(path) else {
        return Vec::new();
    };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let start = len.saturating_sub(64 * 1024);
    let _ = f.seek(SeekFrom::Start(start));
    let mut buf = String::new();
    let _ = f.read_to_string(&mut buf);
    let mut out = Vec::new();
    for line in buf.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains("error") && !lower.contains("attempt") {
            let trimmed = line.trim();
            if trimmed.len() > 12 {
                out.push(trimmed.to_string());
            }
        }
    }
    out.truncate(8);
    out
}

fn scan_once() -> ObservedTransfers {
    let mut track = tracker().lock().unwrap_or_else(|e| e.into_inner());
    let mut transferring = Vec::new();
    let mut queued = 0u64;
    let mut uploading = 0u64;
    let mut dirty_bytes = 0u64;

    for root in cache_root_paths() {
        let (q, u, b) = scan_vfs_meta(&root, &mut track, &mut transferring);
        queued += q;
        uploading += u;
        dirty_bytes += b;
    }

    let mut now_dirty: HashMap<String, i64> = HashMap::new();
    for t in &transferring {
        if t.phase.as_deref() == Some("queued") || t.phase.as_deref() == Some("writing") {
            now_dirty.insert(t.name.clone(), t.size);
        }
    }
    if track.ready {
        let finished: Vec<(String, i64)> = track
            .dirty
            .iter()
            .filter(|(name, _)| !now_dirty.contains_key(*name))
            .map(|(name, size)| (name.clone(), *size))
            .collect();
        for (name, size) in finished {
            crate::history::push_finished(&name, size);
            track.done.push(item(
                name,
                size,
                size,
                0.0,
                "uploaded",
                "disk",
                "cloud",
            ));
        }
    }
    track.ready = true;
    if track.done.len() > 40 {
        let skip = track.done.len() - 40;
        track.done.drain(0..skip);
    }
    track.samples.retain(|key, _| {
        now_dirty.keys().any(|name| key == &format!("dirty:{name}"))
    });
    track.dirty = now_dirty;

    let mut failures = Vec::new();
    for inst in list_instances() {
        if let Some(log) = flag_value(&inst.parsed, "--log-file") {
            failures.extend(log_errors(Path::new(log)));
        }
    }
    crate::history::note_failures(&failures);

    transferring.sort_by(|a, b| {
        b.speed
            .partial_cmp(&a.speed)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let recently_done = track.done.clone();
    drop(track);

    ObservedTransfers {
        transferring,
        queued,
        uploading,
        dirty_bytes,
        recently_done,
    }
}

struct CachedObs {
    at: Instant,
    value: ObservedTransfers,
}

fn observation_cache() -> &'static Mutex<Option<CachedObs>> {
    static CACHE: OnceLock<Mutex<Option<CachedObs>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

fn scanning() -> &'static AtomicBool {
    static SCANNING: AtomicBool = AtomicBool::new(false);
    &SCANNING
}

pub fn cached_observation() -> Option<ObservedTransfers> {
    observation_cache()
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|hit| hit.value.clone()))
}

pub fn observe_transfers() -> ObservedTransfers {
    let fresh = std::time::Duration::from_secs(12);
    if let Some(hit) = observation_cache()
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().filter(|hit| hit.at.elapsed() < fresh).map(|hit| hit.value.clone()))
    {
        return hit;
    }
    if scanning().swap(true, Ordering::AcqRel) {
        return observation_cache()
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(|hit| hit.value.clone()))
            .unwrap_or_else(|| ObservedTransfers {
                transferring: Vec::new(),
                queued: 0,
                uploading: 0,
                dirty_bytes: 0,
                recently_done: Vec::new(),
            });
    }
    let value = scan_once();
    if let Ok(mut guard) = observation_cache().lock() {
        *guard = Some(CachedObs {
            at: Instant::now(),
            value: value.clone(),
        });
    }
    scanning().store(false, Ordering::Release);
    value
}
