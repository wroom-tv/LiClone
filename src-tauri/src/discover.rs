use std::collections::HashMap;

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::Serialize;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use crate::parse::{flag_value, has_flag, parse_argv, rc_addr, split_cmdline, ParsedRclone};
use crate::rclone::hidden_command;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RcloneInstance {
    pub pid: u32,
    pub name: String,
    pub exe: Option<String>,
    pub cwd: Option<String>,
    pub cmd: Vec<String>,
    pub cmd_line: String,
    pub parsed: ParsedRclone,
    pub rc_addr: Option<String>,
    pub rc_user: Option<String>,
    pub rc_has_pass: bool,
    pub cache_dir: Option<String>,
    pub vfs_cache_mode: Option<String>,
    pub cpu: f32,
    pub memory: u64,
    pub run_time_secs: Option<u64>,
    pub status: String,
    pub parent_pid: Option<u32>,
}

fn refresh_kind() -> ProcessRefreshKind {
    ProcessRefreshKind::nothing()
        .with_memory()
        .with_cpu()
        .with_cmd(UpdateKind::Always)
        .with_exe(UpdateKind::Always)
        .with_cwd(UpdateKind::Always)
}

#[derive(Default, Clone)]
struct ExtraProc {
    command_line: Option<String>,
    exe: Option<String>,
}

fn extra_process_info() -> HashMap<u32, ExtraProc> {
    #[cfg(windows)]
    {
        wmi_rclone_processes()
    }
    #[cfg(not(windows))]
    {
        HashMap::new()
    }
}

#[cfg(windows)]
fn wmi_rclone_processes() -> HashMap<u32, ExtraProc> {
    let script = r#"
$ErrorActionPreference='SilentlyContinue'
Get-CimInstance Win32_Process | Where-Object { $_.Name -match 'rclone' } |
  Select-Object ProcessId, CommandLine, ExecutablePath |
  ConvertTo-Json -Compress
"#;
    let mut cmd = hidden_command("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", script]);
    let Ok(out) = cmd.output() else {
        return HashMap::new();
    };
    let raw = String::from_utf8_lossy(&out.stdout);
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return HashMap::new();
    }
    let values: Vec<serde_json::Value> = if trimmed.starts_with('[') {
        serde_json::from_str(trimmed).unwrap_or_default()
    } else {
        serde_json::from_str::<serde_json::Value>(trimmed)
            .ok()
            .into_iter()
            .collect()
    };
    let mut map = HashMap::new();
    for v in values {
        let pid = v.get("ProcessId").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
        if pid == 0 {
            continue;
        }
        map.insert(
            pid,
            ExtraProc {
                command_line: v
                    .get("CommandLine")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty()),
                exe: v
                    .get("ExecutablePath")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty()),
            },
        );
    }
    map
}

pub fn invalidate_instance_cache() {
    if let Ok(mut guard) = instance_cache().lock() {
        *guard = None;
    }
}

pub fn list_instances() -> Vec<RcloneInstance> {
    if let Ok(guard) = instance_cache().lock() {
        if let Some(hit) = guard.as_ref() {
            if hit.at.elapsed() < Duration::from_secs(8) {
                return hit.items.clone();
            }
        }
    }
    let items = scan_instances();
    if let Ok(mut guard) = instance_cache().lock() {
        *guard = Some(InstanceHit {
            at: Instant::now(),
            items: items.clone(),
        });
    }
    items
}

struct InstanceHit {
    at: Instant,
    items: Vec<RcloneInstance>,
}

fn instance_cache() -> &'static Mutex<Option<InstanceHit>> {
    static CACHE: OnceLock<Mutex<Option<InstanceHit>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

pub(crate) fn scan_instances() -> Vec<RcloneInstance> {
    let extras = extra_process_info();
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, refresh_kind());
    let mut out = Vec::new();

    for (pid, proc_) in sys.processes() {
        let name = proc_.name().to_string_lossy().to_string();
        let lower = name.to_lowercase();
        let pid_u = pid.as_u32();
        let extra = extras.get(&pid_u);
        let looks_rclone = lower.contains("rclone")
            || extra
                .and_then(|e| e.exe.as_deref())
                .map(|e| e.to_lowercase().contains("rclone"))
                .unwrap_or(false)
            || extra
                .and_then(|e| e.command_line.as_deref())
                .map(|e| e.to_lowercase().contains("rclone"))
                .unwrap_or(false);
        if !looks_rclone {
            continue;
        }
        if lower.contains("liclone") {
            continue;
        }

        let mut cmd: Vec<String> = proc_
            .cmd()
            .iter()
            .map(|s| s.to_string_lossy().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if cmd.is_empty() {
            if let Some(line) = extra.and_then(|e| e.command_line.clone()) {
                cmd = split_cmdline(&line);
            }
        }
        if cmd.is_empty() {
            if let Some(exe) = proc_.exe().map(|p| p.display().to_string()) {
                cmd.push(exe);
            } else if let Some(exe) = extra.and_then(|e| e.exe.clone()) {
                cmd.push(exe);
            } else {
                cmd.push(name.clone());
            }
        }

        let parsed = parse_argv(&cmd);
        let rc = rc_addr(&parsed);
        let cache_dir = flag_value(&parsed, "--cache-dir").map(|s| s.to_string());
        let vfs_cache_mode = flag_value(&parsed, "--vfs-cache-mode").map(|s| s.to_string());
        let rc_user = flag_value(&parsed, "--rc-user").map(|s| s.to_string());
        let rc_has_pass =
            flag_value(&parsed, "--rc-pass").is_some() || has_flag(&parsed, "--rc-htpasswd");
        let parent_pid = proc_.parent().map(|p: Pid| p.as_u32());
        let exe = proc_
            .exe()
            .map(|p| p.display().to_string())
            .or_else(|| extra.and_then(|e| e.exe.clone()));

        out.push(RcloneInstance {
            pid: pid_u,
            name,
            exe,
            cwd: proc_.cwd().map(|p| p.display().to_string()),
            cmd_line: if cmd.len() == 1 {
                extra
                    .and_then(|e| e.command_line.clone())
                    .unwrap_or_else(|| cmd.join(" "))
            } else {
                cmd.join(" ")
            },
            cmd,
            parsed,
            rc_addr: rc,
            rc_user,
            rc_has_pass,
            cache_dir,
            vfs_cache_mode,
            cpu: proc_.cpu_usage(),
            memory: proc_.memory(),
            run_time_secs: Some(proc_.run_time()),
            status: format!("{:?}", proc_.status()),
            parent_pid,
        });
    }

    out.sort_by_key(|i| i.pid);
    out
}

pub fn stop_pid(pid: u32) -> Result<(), String> {
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, refresh_kind());
    let pid = Pid::from_u32(pid);
    match sys.process(pid) {
        Some(p) => {
            if p.kill() {
                Ok(())
            } else {
                Err(format!("Could not stop process {pid}."))
            }
        }
        None => Err(format!("Process {pid} is not running.")),
    }
}
