use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupState {
    pub completed: bool,
    pub autostart: bool,
}

#[derive(Deserialize)]
struct Saved {
    completed: bool,
}

fn data_file() -> Result<PathBuf, String> {
    let base = dirs::data_dir().ok_or("Could not resolve the app data directory.")?;
    Ok(base.join("liclone").join("setup.json"))
}

fn startup_link() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Could not resolve the home directory.")?;
    Ok(home
        .join("AppData")
        .join("Roaming")
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("Startup")
        .join("LiClone.lnk"))
}

pub fn refresh_tray_launch() {
    if startup_link().map(|path| path.exists()).unwrap_or(false) {
        let _ = set_autostart(true);
    }
}

pub fn status() -> SetupState {
    let completed = data_file()
        .ok()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str::<Saved>(&text).ok())
        .map(|saved| saved.completed)
        .unwrap_or(false);
    let autostart = startup_link().map(|path| path.exists()).unwrap_or(false);
    SetupState { completed, autostart }
}

pub fn complete(autostart: bool) -> Result<SetupState, String> {
    set_autostart(autostart)?;
    let path = data_file()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let body = serde_json::json!({ "completed": true });
    fs::write(&path, body.to_string()).map_err(|e| e.to_string())?;
    Ok(status())
}

fn set_autostart(on: bool) -> Result<(), String> {
    let link = startup_link()?;
    if !on {
        if link.exists() {
            fs::remove_file(&link).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let work = exe
        .parent()
        .map(|dir| dir.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let exe = exe.to_string_lossy().replace('\'', "''");
    let work = work.to_string_lossy().replace('\'', "''");
    let link = link.to_string_lossy().replace('\'', "''");
    let script = format!(
        "$s = (New-Object -ComObject WScript.Shell).CreateShortcut('{link}'); $s.TargetPath = '{exe}'; $s.Arguments = '--tray'; $s.WorkingDirectory = '{work}'; $s.WindowStyle = 7; $s.Description = 'Wroom LiClone'; $s.Save()"
    );
    let mut cmd = crate::rclone::hidden_command("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() && startup_link()?.exists() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if err.is_empty() {
            "Could not add LiClone to startup.".into()
        } else {
            err
        })
    }
}
