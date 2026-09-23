use std::process::Stdio;
use std::time::Duration;

use serde::Deserialize;

use crate::rclone::{find_rclone, hidden_command};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteDraft {
    pub name: String,
    pub kind: String,
    pub params: Vec<RemoteParam>,
    pub obscure: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteParam {
    pub key: String,
    pub value: String,
}

pub fn create_remote(draft: RemoteDraft) -> Result<String, String> {
    let name = draft.name.trim();
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err("Use a remote name with letters, numbers, dashes, or underscores.".into());
    }
    let kind = draft.kind.trim();
    if kind.is_empty() {
        return Err("Pick a remote type.".into());
    }
    let rclone = find_rclone().ok_or("rclone was not found on PATH.")?;
    let mut args = vec!["config".to_string(), "create".into(), name.into(), kind.into()];
    for param in &draft.params {
        let key = param.key.trim();
        let mut value = param.value.trim().to_string();
        if key.is_empty() || value.is_empty() {
            continue;
        }
        if draft.obscure.iter().any(|k| k == key) {
            value = obscure(&rclone, &value)?;
        }
        args.push(format!("{key}={value}"));
    }
    let mut cmd = hidden_command(rclone.to_str().unwrap_or("rclone"));
    cmd.args(&args);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let started = std::time::Instant::now();
    loop {
        if started.elapsed() > Duration::from_secs(120) {
            let _ = child.kill();
            return Err("Connecting the remote took too long. Finish the browser sign-in, then try again.".into());
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut out = String::new();
                if let Some(mut stdout) = child.stdout.take() {
                    let _ = std::io::Read::read_to_string(&mut stdout, &mut out);
                }
                let mut err = String::new();
                if let Some(mut stderr) = child.stderr.take() {
                    let _ = std::io::Read::read_to_string(&mut stderr, &mut err);
                }
                if status.success() {
                    return Ok(format!("{name}:"));
                }
                let detail = if err.trim().is_empty() { out.trim() } else { err.trim() };
                return Err(if detail.is_empty() {
                    "rclone config create failed.".into()
                } else {
                    detail.to_string()
                });
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(200)),
            Err(e) => return Err(e.to_string()),
        }
    }
}

pub(crate) fn obscure(rclone: &std::path::Path, secret: &str) -> Result<String, String> {
    let mut cmd = hidden_command(rclone.to_str().unwrap_or("rclone"));
    cmd.args(["obscure", secret]);
    let out = cmd.output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    let value = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if value.is_empty() {
        Err("rclone obscure returned an empty password.".into())
    } else {
        Ok(value)
    }
}
