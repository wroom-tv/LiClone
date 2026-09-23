use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::Serialize;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RcloneInfo {
    pub found: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub compatible: bool,
    pub error: Option<String>,
}

fn hide(cmd: &mut Command) {
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
}

fn candidate_files() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let names = ["rclone.exe", "rclone"];
    let mut dirs: Vec<PathBuf> = Vec::new();

    if let Ok(path) = env::var("PATH") {
        dirs.extend(env::split_paths(&path));
    }
    dirs.extend(registry_path_dirs());

    if let Some(local) = dirs::data_local_dir() {
        dirs.push(local.join(r"Microsoft\WinGet\Links"));
        dirs.push(local.join(r"Programs\rclone"));
    }
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(r"scoop\shims"));
        dirs.push(home.join(r"scoop\apps\rclone\current"));
        dirs.push(home.join(r"AppData\Local\Microsoft\WinGet\Packages"));
        dirs.push(home.join("bin"));
    }
    dirs.push(PathBuf::from(r"C:\Program Files\rclone"));
    dirs.push(PathBuf::from(r"C:\Program Files (x86)\rclone"));
    dirs.push(PathBuf::from(r"C:\rclone"));
    dirs.push(PathBuf::from(r"C:\ProgramData\chocolatey\bin"));

    for dir in dirs {
        for name in names {
            out.push(dir.join(name));
        }
    }
    out
}

fn registry_path_dirs() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        let script = r#"
$m = [Environment]::GetEnvironmentVariable('Path','Machine')
$u = [Environment]::GetEnvironmentVariable('Path','User')
($m + ';' + $u)
"#;
        let mut cmd = Command::new("powershell");
        hide(&mut cmd);
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", script]);
        if let Ok(out) = cmd.output() {
            let joined = String::from_utf8_lossy(&out.stdout);
            return env::split_paths(joined.trim()).collect();
        }
    }
    Vec::new()
}

fn rclone_supported(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let Some(pos) = lower.find("v1.").or_else(|| lower.find(" v1")) else {
        return false;
    };
    let rest = &lower[pos + 1..];
    let mut nums = rest.split(|c: char| !c.is_ascii_digit()).filter(|p| !p.is_empty());
    let major: u32 = nums.next().unwrap_or("0").parse().unwrap_or(0);
    let minor: u32 = nums.next().unwrap_or("0").parse().unwrap_or(0);
    major > 1 || (major == 1 && minor >= 65)
}

fn looks_like_rclone(path: &Path) -> bool {
    path.is_file()
        && path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.eq_ignore_ascii_case("rclone"))
            .unwrap_or(false)
}

pub fn find_rclone() -> Option<PathBuf> {
    if let Ok(p) = which::which("rclone") {
        return Some(p);
    }
    if let Ok(p) = which::which("rclone.exe") {
        return Some(p);
    }
    if let Some(p) = candidate_files().into_iter().find(|p| looks_like_rclone(p)) {
        return Some(p);
    }

    // Last resort: a running rclone.exe (works even when PATH is stale).
    crate::discover::list_instances()
        .into_iter()
        .filter_map(|i| i.exe)
        .map(PathBuf::from)
        .find(|p| looks_like_rclone(p))
}

pub fn rclone_info() -> RcloneInfo {
    if let Ok(guard) = info_cache().lock() {
        if let Some(hit) = guard.as_ref() {
            if hit.at.elapsed() < Duration::from_secs(45) {
                return hit.value.clone();
            }
        }
    }
    let value = load_rclone_info();
    if let Ok(mut guard) = info_cache().lock() {
        *guard = Some(InfoHit {
            at: Instant::now(),
            value: value.clone(),
        });
    }
    value
}

struct InfoHit {
    at: Instant,
    value: RcloneInfo,
}

fn info_cache() -> &'static Mutex<Option<InfoHit>> {
    static CACHE: OnceLock<Mutex<Option<InfoHit>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

fn load_rclone_info() -> RcloneInfo {
    let Some(path) = find_rclone() else {
        return RcloneInfo {
            found: false,
            path: None,
            version: None,
            compatible: false,
            error: Some("rclone was not found on PATH.".into()),
        };
    };
    let mut cmd = Command::new(&path);
    cmd.arg("version");
    hide(&mut cmd);
    match cmd.output() {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let version = stdout
                .lines()
                .chain(stderr.lines())
                .map(|l| l.trim())
                .find(|l| l.to_lowercase().contains("rclone"))
                .map(|l| l.to_string())
                .or_else(|| {
                    stdout
                        .lines()
                        .next()
                        .map(|l| l.trim().to_string())
                        .filter(|s| !s.is_empty())
                });
            RcloneInfo {
                found: true,
                path: Some(path.display().to_string()),
                version: version.clone(),
                compatible: version.as_deref().map(rclone_supported).unwrap_or(false),
                error: None,
            }
        }
        Err(e) => RcloneInfo {
            found: true,
            path: Some(path.display().to_string()),
            version: None,
            compatible: false,
            error: Some(e.to_string()),
        },
    }
}

pub fn list_remotes() -> Result<Vec<String>, String> {
    let path = find_rclone().ok_or_else(|| "rclone was not found on PATH.".to_string())?;
    let mut cmd = Command::new(&path);
    cmd.arg("listremotes");
    hide(&mut cmd);
    let out = cmd.output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(if err.trim().is_empty() {
            "rclone listremotes failed.".into()
        } else {
            err.trim().to_string()
        });
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    Ok(stdout
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

pub fn install_rclone() -> Result<RcloneInfo, String> {
    let mut cmd = hidden_command("winget");
    cmd.args([
        "install",
        "--id",
        "Rclone.Rclone",
        "-e",
        "--scope",
        "user",
        "--accept-package-agreements",
        "--accept-source-agreements",
        "--disable-interactivity",
    ]);
    let out = cmd.output().map_err(|e| {
        format!("Could not start winget ({e}). Install rclone from https://rclone.org/downloads/ and add it to PATH.")
    })?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let text = format!("{stdout}\n{stderr}").to_lowercase();
    let already = text.contains("already installed") || text.contains("no available upgrade");
    if !out.status.success() && !already {
        let detail = format!("{}\n{}", stdout.trim(), stderr.trim());
        return Err(detail.trim().to_string());
    }
    if let Ok(mut guard) = info_cache().lock() {
        *guard = None;
    }
    let Some(path) = find_rclone() else {
        return Err("rclone was installed, but LiClone cannot see it yet. Close and open LiClone, or add rclone to PATH.".into());
    };
    if let Some(dir) = path.parent() {
        let _ = ensure_user_path(dir);
        prepend_process_path(dir);
    }
    if let Ok(mut guard) = info_cache().lock() {
        *guard = None;
    }
    Ok(rclone_info())
}

fn prepend_process_path(dir: &Path) {
    let dir = dir.display().to_string();
    let current = env::var("PATH").unwrap_or_default();
    if !current.split(';').any(|part| part.eq_ignore_ascii_case(&dir)) {
        env::set_var("PATH", format!("{dir};{current}"));
    }
}

fn ensure_user_path(dir: &Path) -> Result<(), String> {
    let dir = dir.display().to_string().replace('\'', "''");
    let script = format!(
        r#"
$dir = '{dir}'
$user = [Environment]::GetEnvironmentVariable('Path','User')
$parts = @()
if ($user) {{ $parts = @($user -split ';' | Where-Object {{ $_ -and ($_.TrimEnd('\') -ine $dir.TrimEnd('\')) }}) }}
[Environment]::SetEnvironmentVariable('Path', ((@($dir) + $parts) -join ';'), 'User')
"#
    );
    let mut cmd = hidden_command("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn hidden_command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    hide(&mut cmd);
    cmd
}
