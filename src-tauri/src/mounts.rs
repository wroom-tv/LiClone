use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::parse::quote_win;
use crate::rclone::{find_rclone, hidden_command};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MountProfile {
    pub id: String,
    pub name: String,
    pub remote: String,
    pub remote_path: String,
    pub mount_point: String,
    pub vfs_cache_mode: String,
    pub cache_dir: String,
    pub extra_args: String,
    pub rc_enabled: bool,
    pub rc_port: u16,
    pub network_mode: bool,
    pub read_only: bool,
    pub vol_name: String,
    pub start_on_login: bool,
    pub background: bool,
    /// rclone flags without the leading dashes. Booleans use "1".
    #[serde(default)]
    pub settings: BTreeMap<String, String>,
}

impl MountProfile {
    pub fn source_spec(&self) -> String {
        let remote = self.remote.trim();
        let remote = remote.strip_suffix(':').unwrap_or(remote);
        let path = self.remote_path.trim().trim_start_matches('/');
        if path.is_empty() {
            format!("{remote}:")
        } else {
            format!("{remote}:{path}")
        }
    }

    pub fn argv(&self, rclone: &Path) -> Vec<String> {
        let mut args = vec![
            rclone.display().to_string(),
            "mount".into(),
            self.source_spec(),
            self.mount_point.trim().to_string(),
        ];
        let flags = flag_map(self);
        let rc_on = truthy(flags.get("rc").map(String::as_str).unwrap_or(""));
        let port = flags
            .get("rc-port")
            .and_then(|v| v.parse::<u16>().ok())
            .filter(|p| *p != 0)
            .unwrap_or(if self.rc_port == 0 { 5572 } else { self.rc_port });
        for (key, value) in &flags {
            if key == "rc" || key == "rc-port" {
                continue;
            }
            let value = value.trim();
            if SWITCHES.contains(&key.as_str()) {
                if truthy(value) {
                    args.push(format!("--{key}"));
                }
                continue;
            }
            if value.is_empty() {
                continue;
            }
            args.push(format!("--{key}"));
            args.push(value.to_string());
        }
        if rc_on {
            args.push("--rc".into());
            args.push("--rc-no-auth".into());
            args.push("--rc-addr".into());
            args.push(format!("127.0.0.1:{port}"));
        }
        for token in split_args(&self.extra_args) {
            args.push(token);
        }
        args
    }
}

const SWITCHES: &[&str] = &[
    "network-mode",
    "read-only",
    "fast-list",
    "links",
    "vfs-case-insensitive",
    "no-modtime",
    "no-checksum",
    "allow-other",
    "daemon",
    "use-mmap",
];

fn truthy(value: &str) -> bool {
    matches!(value.trim(), "1" | "true" | "yes" | "on")
}

fn flag_map(profile: &MountProfile) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    map.insert("vfs-cache-mode".into(), profile.vfs_cache_mode.clone());
    map.insert("cache-dir".into(), profile.cache_dir.clone());
    map.insert("volname".into(), profile.vol_name.clone());
    map.insert(
        "network-mode".into(),
        if profile.network_mode { "1" } else { "" }.into(),
    );
    map.insert(
        "read-only".into(),
        if profile.read_only { "1" } else { "" }.into(),
    );
    map.insert(
        "rc".into(),
        if profile.rc_enabled { "1" } else { "" }.into(),
    );
    map.insert("rc-port".into(), profile.rc_port.to_string());
    for (key, value) in &profile.settings {
        map.insert(key.clone(), value.clone());
    }
    map
}

fn sync_legacy(profile: &mut MountProfile) {
    if let Some(v) = profile.settings.get("vfs-cache-mode") {
        profile.vfs_cache_mode = v.clone();
    }
    if let Some(v) = profile.settings.get("cache-dir") {
        profile.cache_dir = v.clone();
    }
    if let Some(v) = profile.settings.get("volname") {
        profile.vol_name = v.clone();
    }
    if let Some(v) = profile.settings.get("network-mode") {
        profile.network_mode = truthy(v);
    }
    if let Some(v) = profile.settings.get("read-only") {
        profile.read_only = truthy(v);
    }
    if let Some(v) = profile.settings.get("rc") {
        profile.rc_enabled = truthy(v);
    }
    if let Some(v) = profile.settings.get("rc-port") {
        if let Ok(port) = v.parse::<u16>() {
            if port != 0 {
                profile.rc_port = port;
            }
        }
    }
}

fn split_args(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    for c in raw.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => cur.push(c),
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c.is_whitespace() => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            None => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn data_dir() -> Result<PathBuf, String> {
    let base = dirs::data_dir().ok_or("Could not resolve app data directory.")?;
    let dir = base.join("liclone");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn profiles_path() -> Result<PathBuf, String> {
    Ok(data_dir()?.join("mounts.json"))
}

pub fn load_profiles() -> Result<Vec<MountProfile>, String> {
    let path = profiles_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

fn save_all(profiles: &[MountProfile]) -> Result<(), String> {
    let path = profiles_path()?;
    let json = serde_json::to_string_pretty(profiles).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

pub fn upsert_profile(mut profile: MountProfile) -> Result<MountProfile, String> {
    if profile.id.is_empty() {
        profile.id = Uuid::new_v4().to_string();
    }
    if profile.rc_port == 0 {
        profile.rc_port = 5572;
    }
    if profile.vfs_cache_mode.is_empty() {
        profile.vfs_cache_mode = "full".into();
    }
    sync_legacy(&mut profile);
    assign_rc_port(&mut profile);
    let mut profiles = load_profiles()?;
    if let Some(existing) = profiles.iter_mut().find(|p| p.id == profile.id) {
        *existing = profile.clone();
    } else {
        profiles.push(profile.clone());
    }
    save_all(&profiles)?;
    sync_startup(&profile)?;
    Ok(profile)
}

pub fn delete_profile(id: &str) -> Result<(), String> {
    let mut profiles = load_profiles()?;
    profiles.retain(|p| p.id != id);
    save_all(&profiles)?;
    remove_startup(id);
    Ok(())
}

fn start_lock() -> &'static std::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
}

pub fn start_mount(id: &str) -> Result<u32, String> {
    let _guard = start_lock().lock().map_err(|e| e.to_string())?;
    let profiles = load_profiles()?;
    let profile = profiles
        .iter()
        .find(|p| p.id == id)
        .ok_or("Mount profile not found.")?
        .clone();
    let rclone = find_rclone().ok_or("rclone was not found on PATH.")?;
    if profile.mount_point.trim().is_empty() {
        return Err("Set a mount point first.".into());
    }
    refuse_double_mount(&profile)?;
    let mut profile = profile;
    let before = profile.rc_port;
    assign_rc_port(&mut profile);
    if profile.rc_port != before {
        let _ = persist_started(&profile);
    }
    let mut argv = profile.argv(&rclone);
    crate::schedule::append_bwlimit(&mut argv);
    let mut cmd = hidden_command(&argv[0]);
    cmd.args(&argv[1..]);
    if !profile.background {
        // still no extra console; Tauri owns the UI
    }
    let child = cmd.spawn().map_err(|e| {
        format!(
            "Failed to start rclone: {e}. On Windows, rclone mount needs WinFSP."
        )
    })?;
    crate::discover::invalidate_instance_cache();
    Ok(child.id())
}

fn refuse_double_mount(profile: &MountProfile) -> Result<(), String> {
    let point = norm_point(&profile.mount_point);
    let source = norm_source(&profile.source_spec());
    for inst in crate::discover::scan_instances() {
        if inst.parsed.verb != "mount" {
            continue;
        }
        if let Some(existing) = inst.parsed.mount_point.as_deref() {
            if norm_point(existing) == point {
                return Err(format!("{} is already mounted.", profile.mount_point.trim()));
            }
        }
        if let Some(existing) = inst.parsed.remote.as_deref() {
            if norm_source(existing) == source {
                return Err(format!("{} is already mounted.", profile.source_spec()));
            }
        }
    }
    Ok(())
}

fn persist_started(profile: &MountProfile) -> Result<(), String> {
    let mut profiles = load_profiles()?;
    if let Some(existing) = profiles.iter_mut().find(|p| p.id == profile.id) {
        *existing = profile.clone();
        save_all(&profiles)?;
        sync_startup(profile)?;
    }
    Ok(())
}

fn assign_rc_port(profile: &mut MountProfile) {
    let enabled = profile.rc_enabled
        || profile
            .settings
            .get("rc")
            .map(|value| truthy(value))
            .unwrap_or(false);
    if !enabled {
        return;
    }
    let preferred = profile
        .settings
        .get("rc-port")
        .and_then(|value| value.parse::<u16>().ok())
        .filter(|port| *port > 0)
        .unwrap_or(if profile.rc_port == 0 { 5572 } else { profile.rc_port });
    let port = free_port(preferred, &profile.id);
    profile.rc_enabled = true;
    profile.rc_port = port;
    profile.settings.insert("rc".into(), "1".into());
    profile.settings.insert("rc-port".into(), port.to_string());
}

fn free_port(preferred: u16, except_id: &str) -> u16 {
    let used = used_ports(except_id);
    let mut port = if preferred == 0 { 5572 } else { preferred };
    for _ in 0..2000 {
        if !used.contains(&port) {
            return port;
        }
        port = port.saturating_add(1);
        if port < 1024 {
            port = 5572;
        }
    }
    port
}

fn used_ports(except_id: &str) -> std::collections::HashSet<u16> {
    let mut used = std::collections::HashSet::new();
    if let Ok(profiles) = load_profiles() {
        for profile in profiles {
            if profile.id == except_id {
                continue;
            }
            let enabled = profile.rc_enabled
                || profile
                    .settings
                    .get("rc")
                    .map(|value| truthy(value))
                    .unwrap_or(false);
            if enabled && profile.rc_port > 0 {
                used.insert(profile.rc_port);
            }
            if let Some(port) = profile
                .settings
                .get("rc-port")
                .and_then(|value| value.parse::<u16>().ok())
            {
                if port > 0 {
                    used.insert(port);
                }
            }
        }
    }
    for inst in crate::discover::scan_instances() {
        if let Some(addr) = inst.rc_addr.as_deref() {
            if let Some(port) = addr.rsplit(':').next().and_then(|value| value.parse::<u16>().ok()) {
                if port > 0 {
                    used.insert(port);
                }
            }
        }
    }
    used
}

fn norm_point(value: &str) -> String {
    value
        .trim()
        .trim_end_matches(['\\', '/'])
        .to_ascii_lowercase()
}

fn norm_source(value: &str) -> String {
    let value = value.trim().trim_end_matches(['\\', '/']).replace('\\', "/");
    let (remote, path) = value.split_once(':').unwrap_or((value.as_str(), ""));
    let path = path.trim_matches('/');
    format!("{}:{}", remote.to_ascii_lowercase(), path.to_ascii_lowercase())
}

const CACHE_CAP_GB: u64 = 100;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheAdvice {
    pub value: String,
    pub label: String,
    pub reason: String,
}

pub fn cache_advice() -> CacheAdvice {
    let Some(free) = free_bytes_for_home() else {
        return CacheAdvice {
            value: "20G".into(),
            label: "20 GB".into(),
            reason: format!("20 GB when the free space on this PC could not be read. Never above {CACHE_CAP_GB} GB."),
        };
    };
    let gb = 1024 * 1024 * 1024;
    let free_gb = free / gb;
    let share = (free_gb / 10).clamp(1, CACHE_CAP_GB);
    CacheAdvice {
        value: format!("{share}G"),
        label: format!("{share} GB"),
        reason: format!("10% of the {free_gb} GB free on this PC, and never above {CACHE_CAP_GB} GB."),
    }
}

fn free_bytes_for_home() -> Option<u64> {
    let home = dirs::home_dir()?;
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut best: Option<(usize, u64)> = None;
    for disk in &disks {
        let mount = disk.mount_point();
        if !home.starts_with(mount) {
            continue;
        }
        let len = mount.as_os_str().len();
        let space = disk.available_space();
        if space == 0 {
            continue;
        }
        if best.as_ref().map(|(n, _)| len > *n).unwrap_or(true) {
            best = Some((len, space));
        }
    }
    best.map(|(_, space)| space)
}

pub fn preview_command(profile: MountProfile) -> Result<String, String> {
    let rclone = find_rclone().ok_or("rclone was not found on PATH.")?;
    let mut argv = profile.argv(&rclone);
    crate::schedule::append_bwlimit(&mut argv);
    Ok(argv.iter().map(|a| quote_win(a)).collect::<Vec<_>>().join(" "))
}

fn startup_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| {
        h.join("AppData")
            .join("Roaming")
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("Startup")
    })
}

fn startup_script(id: &str) -> Option<PathBuf> {
    startup_dir().map(|d| d.join(format!("LiClone-{id}.vbs")))
}

fn remove_startup(id: &str) {
    if let Some(path) = startup_script(id) {
        let _ = fs::remove_file(path);
    }
}

fn sync_startup(profile: &MountProfile) -> Result<(), String> {
    if !profile.start_on_login {
        remove_startup(&profile.id);
        return Ok(());
    }
    let rclone = find_rclone().ok_or("rclone was not found on PATH.")?;
    let argv = profile.argv(&rclone);
    let quoted: Vec<String> = argv.iter().map(|a| quote_win(a)).collect();
    let line = quoted.join(" ");
    let escaped = line.replace('"', "\"\"");
    let vbs = format!(
        "Set sh = CreateObject(\"Wscript.Shell\")\r\nsh.Run \"{escaped}\", 0, False\r\n"
    );
    let path = startup_script(&profile.id).ok_or("Could not resolve Startup folder.")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut f = fs::File::create(&path).map_err(|e| e.to_string())?;
    f.write_all(vbs.as_bytes()).map_err(|e| e.to_string())?;
    Ok(())
}
