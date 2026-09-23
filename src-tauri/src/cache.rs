use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use walkdir::WalkDir;

use crate::discover::list_instances;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheFile {
    pub path: String,
    pub relative: String,
    pub size: u64,
    pub mtime: u64,
    pub recent: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheBucket {
    pub remote: String,
    pub files: u64,
    pub bytes: u64,
    pub recent_files: u64,
    pub recent_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheRoot {
    pub path: String,
    pub exists: bool,
    pub bytes: u64,
    pub files: u64,
    pub dirs: u64,
    pub buckets: Vec<CacheBucket>,
    pub largest: Vec<CacheFile>,
    pub recent: Vec<CacheFile>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheReport {
    pub roots: Vec<CacheRoot>,
    pub total_bytes: u64,
    pub total_files: u64,
}

fn default_cache_dirs() -> Vec<(PathBuf, String)> {
    let mut dirs = Vec::new();
    if let Some(local) = dirs::data_local_dir() {
        dirs.push((local.join("rclone"), "default local app data".into()));
    }
    if let Some(home) = dirs::home_dir() {
        dirs.push((home.join(".cache").join("rclone"), "XDG cache".into()));
        dirs.push((home.join("AppData").join("Local").join("rclone"), "Windows Local".into()));
    }
    dirs
}

fn unix_mtime(path: &Path) -> u64 {
    path.metadata()
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn scan_root(root: &Path, source: &str) -> CacheRoot {
    let exists = root.exists();
    if !exists {
        return CacheRoot {
            path: root.display().to_string(),
            exists: false,
            bytes: 0,
            files: 0,
            dirs: 0,
            buckets: Vec::new(),
            largest: Vec::new(),
            recent: Vec::new(),
            source: source.to_string(),
        };
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let recent_cut = now.saturating_sub(30 * 60);

    let vfs = if root.join("vfs").is_dir() {
        root.join("vfs")
    } else {
        root.to_path_buf()
    };

    let mut bytes = 0u64;
    let mut files = 0u64;
    let mut dir_count = 0u64;
    let mut bucket_map: HashMap<String, CacheBucket> = HashMap::new();
    let mut all_files: Vec<CacheFile> = Vec::new();

    for entry in WalkDir::new(&vfs)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if entry.file_type().is_dir() {
            dir_count += 1;
            continue;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        bytes += size;
        files += 1;
        let mtime = unix_mtime(path);
        let recent = mtime >= recent_cut;
        let relative = path
            .strip_prefix(&vfs)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let remote = relative
            .split('/')
            .next()
            .unwrap_or("(root)")
            .to_string();
        let bucket = bucket_map.entry(remote.clone()).or_insert(CacheBucket {
            remote,
            files: 0,
            bytes: 0,
            recent_files: 0,
            recent_bytes: 0,
        });
        bucket.files += 1;
        bucket.bytes += size;
        if recent {
            bucket.recent_files += 1;
            bucket.recent_bytes += size;
        }
        all_files.push(CacheFile {
            path: path.display().to_string(),
            relative,
            size,
            mtime,
            recent,
        });
    }

    let mut buckets: Vec<CacheBucket> = bucket_map.into_values().collect();
    buckets.sort_by(|a, b| b.bytes.cmp(&a.bytes));

    let mut largest = all_files.clone();
    largest.sort_by(|a, b| b.size.cmp(&a.size));
    largest.truncate(12);

    let mut recent: Vec<CacheFile> = all_files.into_iter().filter(|f| f.recent).collect();
    recent.sort_by(|a, b| b.mtime.cmp(&a.mtime));
    recent.truncate(20);

    CacheRoot {
        path: root.display().to_string(),
        exists: true,
        bytes,
        files,
        dirs: dir_count,
        buckets,
        largest,
        recent,
        source: source.to_string(),
    }
}

pub fn cache_root_paths() -> Vec<PathBuf> {
    let mut seen = HashMap::<String, PathBuf>::new();
    for (path, _) in default_cache_dirs() {
        seen.entry(path.display().to_string()).or_insert(path);
    }
    for inst in list_instances() {
        if let Some(dir) = inst.cache_dir {
            let p = PathBuf::from(&dir);
            seen.entry(dir).or_insert(p);
        }
    }
    seen.into_values().collect()
}

pub fn analyze_cache() -> CacheReport {
    let mut seen = HashMap::<String, String>::new();
    for (path, source) in default_cache_dirs() {
        seen.entry(path.display().to_string())
            .or_insert(source);
    }
    for inst in list_instances() {
        if let Some(dir) = inst.cache_dir {
            seen.entry(dir).or_insert_with(|| format!("pid {}", inst.pid));
        }
    }

    let mut roots: Vec<CacheRoot> = seen
        .into_iter()
        .map(|(path, source)| scan_root(Path::new(&path), &source))
        .filter(|r| r.exists || r.source.starts_with("pid"))
        .collect();
    roots.sort_by(|a, b| b.bytes.cmp(&a.bytes));

    let total_bytes = roots.iter().map(|r| r.bytes).sum();
    let total_files = roots.iter().map(|r| r.files).sum();
    CacheReport {
        roots,
        total_bytes,
        total_files,
    }
}
