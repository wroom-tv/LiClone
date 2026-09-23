use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

use crate::cache::cache_root_paths;
use crate::rclone::{find_rclone, hidden_command};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheSample {
    pub path: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CachePreview {
    pub mode: String,
    pub files: u64,
    pub bytes: u64,
    pub kept_uploads: u64,
    pub kept_other: u64,
    pub sample: Vec<CacheSample>,
    pub errors: Vec<String>,
    pub cancelled: bool,
}

#[derive(Clone)]
struct Candidate {
    data: PathBuf,
    meta: PathBuf,
    label: String,
    bytes: u64,
}

struct Plan {
    candidates: Vec<Candidate>,
}

static RUNNING: AtomicBool = AtomicBool::new(false);
static CANCEL: AtomicBool = AtomicBool::new(false);

fn plan_slot() -> &'static Mutex<Option<Plan>> {
    static PLAN: OnceLock<Mutex<Option<Plan>>> = OnceLock::new();
    PLAN.get_or_init(|| Mutex::new(None))
}

pub fn cancel() {
    CANCEL.store(true, Ordering::Release);
}

pub fn preview(app: AppHandle, mode: String, older_than_days: u32) -> Result<CachePreview, String> {
    if RUNNING.swap(true, Ordering::AcqRel) {
        return Err("A cache scan is already running.".into());
    }
    CANCEL.store(false, Ordering::Release);
    let result = preview_inner(&app, &mode, older_than_days);
    RUNNING.store(false, Ordering::Release);
    result
}

pub fn apply(app: AppHandle) -> Result<CachePreview, String> {
    if RUNNING.swap(true, Ordering::AcqRel) {
        return Err("A cache scan is already running.".into());
    }
    CANCEL.store(false, Ordering::Release);
    let result = apply_inner(&app);
    RUNNING.store(false, Ordering::Release);
    result
}

fn preview_inner(app: &AppHandle, mode: &str, older_than_days: u32) -> Result<CachePreview, String> {
    let mode = match mode {
        "cloud" | "clean" | "old" => mode,
        _ => return Err("Pick a clear option first.".into()),
    };
    let mut preview = CachePreview {
        mode: mode.into(),
        files: 0,
        bytes: 0,
        kept_uploads: 0,
        kept_other: 0,
        sample: Vec::new(),
        errors: Vec::new(),
        cancelled: false,
    };
    let mut candidates = Vec::new();

    for root in cache_root_paths() {
        if CANCEL.load(Ordering::Acquire) {
            preview.cancelled = true;
            break;
        }
        let vfs = root.join("vfs");
        let meta_root = root.join("vfsMeta");
        if !vfs.is_dir() {
            continue;
        }
        let protected = protected_paths(&meta_root);
        preview.kept_uploads += protected.len() as u64;
        if mode == "cloud" {
            collect_size_check(app, &vfs, &meta_root, &protected, &mut candidates, &mut preview);
        } else {
            collect_local(
                app,
                &vfs,
                &meta_root,
                &protected,
                mode,
                older_than_days.max(1),
                &mut candidates,
                &mut preview,
            );
        }
    }

    if let Ok(mut slot) = plan_slot().lock() {
        *slot = if preview.cancelled {
            None
        } else {
            Some(Plan { candidates })
        };
    }
    emit_preview(app, &preview, if preview.cancelled { "Scan cancelled." } else { "Preview ready. Nothing has been deleted." });
    Ok(preview)
}

fn apply_inner(app: &AppHandle) -> Result<CachePreview, String> {
    let candidates = plan_slot()
        .lock()
        .map_err(|e| e.to_string())?
        .take()
        .map(|plan| plan.candidates)
        .ok_or("Preview the files first. Nothing is deleted until you confirm.")?;

    let mut removed = CachePreview {
        mode: "delete".into(),
        files: 0,
        bytes: 0,
        kept_uploads: 0,
        kept_other: 0,
        sample: Vec::new(),
        errors: Vec::new(),
        cancelled: false,
    };
    for candidate in candidates {
        if CANCEL.load(Ordering::Acquire) {
            removed.cancelled = true;
            break;
        }
        if still_protected(&candidate.meta) {
            removed.kept_uploads += 1;
            continue;
        }
        let bytes = fs::metadata(&candidate.data).map(|m| m.len()).unwrap_or(candidate.bytes);
        if fs::remove_file(&candidate.data).is_err() {
            removed.kept_other += 1;
            continue;
        }
        let _ = fs::remove_file(&candidate.meta);
        removed.files += 1;
        removed.bytes += bytes;
        if removed.sample.len() < 40 {
            removed.sample.push(CacheSample {
                path: candidate.label,
                bytes,
            });
        }
        if removed.files % 50 == 0 {
            emit_preview(app, &removed, "Deleting the previewed files.");
        }
    }
    emit_preview(
        app,
        &removed,
        if removed.cancelled {
            "Delete stopped. Files already removed stay removed."
        } else {
            "Deleted the previewed files. Uploads that changed during the scan were kept."
        },
    );
    Ok(removed)
}

fn collect_local(
    app: &AppHandle,
    vfs: &Path,
    meta_root: &Path,
    protected: &HashSet<String>,
    mode: &str,
    older_than_days: u32,
    candidates: &mut Vec<Candidate>,
    preview: &mut CachePreview,
) {
    let max_age = Duration::from_secs(u64::from(older_than_days) * 86_400);
    for entry in WalkDir::new(vfs).follow_links(false).into_iter().filter_map(|e| e.ok()) {
        if CANCEL.load(Ordering::Acquire) {
            preview.cancelled = true;
            break;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(vfs)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .replace('\\', "/");
        if protected.contains(&rel) {
            continue;
        }
        if mode == "old" {
            let old_enough = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| SystemTime::now().duration_since(t).ok())
                .map(|age| age > max_age)
                .unwrap_or(false);
            if !old_enough {
                preview.kept_other += 1;
                continue;
            }
        }
        push_candidate(meta_root, entry.path(), rel, entry.metadata().map(|m| m.len()).unwrap_or(0), candidates, preview);
        if preview.files % 200 == 0 {
            emit_preview(app, preview, "Scanning cache. Uploads are excluded.");
        }
    }
}

fn collect_size_check(
    app: &AppHandle,
    vfs: &Path,
    meta_root: &Path,
    protected: &HashSet<String>,
    candidates: &mut Vec<Candidate>,
    preview: &mut CachePreview,
) {
    let Some(rclone) = find_rclone() else {
        preview.errors.push("rclone was not found on PATH.".into());
        return;
    };
    let Ok(remotes) = fs::read_dir(vfs) else {
        return;
    };
    for entry in remotes.filter_map(|e| e.ok()) {
        if preview.cancelled || CANCEL.load(Ordering::Acquire) {
            preview.cancelled = true;
            break;
        }
        if !entry.path().is_dir() {
            continue;
        }
        let remote = entry.file_name().to_string_lossy().to_string();
        emit_preview(
            app,
            preview,
            &format!("Checking {remote} with rclone check --size-only --one-way."),
        );
        if let Err(err) = check_sizes(&rclone, &entry.path(), &remote, meta_root, protected, app, candidates, preview) {
            preview.errors.push(format!("{remote}: {err}"));
        }
    }
}

fn check_sizes(
    rclone: &Path,
    local: &Path,
    remote: &str,
    meta_root: &Path,
    protected: &HashSet<String>,
    app: &AppHandle,
    candidates: &mut Vec<Candidate>,
    preview: &mut CachePreview,
) -> Result<(), String> {
    let mut cmd = hidden_command(rclone.to_str().unwrap_or("rclone"));
    cmd.args([
        "check",
        &local.display().to_string(),
        &format!("{remote}:"),
        "--size-only",
        "--one-way",
        "--combined",
        "-",
    ]);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let pid = child.id();
    let stdout = child.stdout.take().ok_or("Could not read rclone check output.")?;
    let stderr = child.stderr.take();
    let err_log = std::thread::spawn(move || {
        let Some(stderr) = stderr else {
            return String::new();
        };
        let mut tail = String::new();
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if tail.len() > 4000 {
                tail = tail.split_off(tail.len().saturating_sub(2000));
            }
            tail.push_str(line.trim());
            tail.push('\n');
        }
        tail
    });
    let mut saw = false;
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        if CANCEL.load(Ordering::Acquire) {
            preview.cancelled = true;
            kill_pid(pid);
            break;
        }
        let line = line.trim();
        let mut chars = line.chars();
        let Some(op) = chars.next() else { continue };
        if !matches!(op, '=' | '*' | '+' | '-' | '!') {
            continue;
        }
        saw = true;
        let path = chars.as_str().trim().trim_start_matches(['/', '\\']).replace('\\', "/");
        if path.is_empty() {
            continue;
        }
        let key = format!("{remote}/{path}");
        if protected.contains(&key) {
            continue;
        }
        if op != '=' {
            preview.kept_other += 1;
            continue;
        }
        let data = child_path(local, &path);
        if !data.is_file() {
            continue;
        }
        let bytes = fs::metadata(&data).map(|m| m.len()).unwrap_or(0);
        push_candidate(meta_root, &data, key, bytes, candidates, preview);
        if preview.files % 25 == 0 {
            emit_preview(app, preview, &format!("{remote}: sizes match the cloud."));
        }
    }
    let err_text = err_log.join().unwrap_or_default();
    let status = child.wait().map_err(|e| e.to_string())?;
    if preview.cancelled {
        return Ok(());
    }
    if !status.success() && !saw {
        let detail = err_text.trim();
        return Err(if detail.is_empty() {
            "rclone check failed.".into()
        } else {
            detail.to_string()
        });
    }
    Ok(())
}

fn push_candidate(
    meta_root: &Path,
    data: &Path,
    key: String,
    bytes: u64,
    candidates: &mut Vec<Candidate>,
    preview: &mut CachePreview,
) {
    preview.files += 1;
    preview.bytes += bytes;
    if preview.sample.len() < 40 {
        preview.sample.push(CacheSample {
            path: key.clone(),
            bytes,
        });
    }
    candidates.push(Candidate {
        data: data.to_path_buf(),
        meta: child_path(meta_root, &key),
        label: key,
        bytes,
    });
}

fn emit_preview(app: &AppHandle, preview: &CachePreview, message: &str) {
    let _ = app.emit(
        "cache-purge",
        serde_json::json!({
            "mode": preview.mode,
            "message": message,
            "files": preview.files,
            "bytes": preview.bytes,
            "keptUploads": preview.kept_uploads,
            "keptOther": preview.kept_other,
            "sample": preview.sample,
            "errors": preview.errors,
            "cancelled": preview.cancelled,
        }),
    );
}

fn protected_paths(meta_root: &Path) -> HashSet<String> {
    let mut protected = HashSet::new();
    if !meta_root.is_dir() {
        return protected;
    }
    for entry in WalkDir::new(meta_root).follow_links(false).into_iter().filter_map(|e| e.ok()) {
        if CANCEL.load(Ordering::Acquire) {
            break;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(meta_root)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .replace('\\', "/");
        if still_protected(entry.path()) {
            protected.insert(rel);
        }
    }
    protected
}

fn still_protected(meta: &Path) -> bool {
    let Ok(text) = fs::read_to_string(meta) else {
        return true;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return true;
    };
    // Dirty means rclone still has local bytes that have not reached the cloud.
    // A partly downloaded file is Dirty false: the full file is already in the cloud.
    value
        .get("Dirty")
        .or_else(|| value.get("dirty"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn kill_pid(pid: u32) {
    let mut cmd = Command::new("taskkill");
    cmd.args(["/F", "/PID", &pid.to_string()]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let _ = cmd.output();
}

fn child_path(root: &Path, rel: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    for part in rel.split(['/', '\\']) {
        if !part.is_empty() && part != "." {
            path.push(part);
        }
    }
    path
}
