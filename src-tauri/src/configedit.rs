use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::rclone::{find_rclone, hidden_command};
use crate::wizard::{self, RemoteParam};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderOption {
    pub name: String,
    pub help: String,
    pub default_value: String,
    pub required: bool,
    pub password: bool,
    pub sensitive: bool,
    pub advanced: bool,
    #[serde(rename = "type")]
    pub type_name: String,
    pub examples: Vec<ProviderExample>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderExample {
    pub value: String,
    pub help: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub name: String,
    pub description: String,
    pub options: Vec<ProviderOption>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteConfig {
    pub name: String,
    pub kind: String,
    pub values: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteParamWrite {
    pub key: String,
    pub value: String,
    pub obscure: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteWrite {
    pub name: String,
    pub previous_name: String,
    pub kind: String,
    pub previous_kind: String,
    pub create: bool,
    pub params: Vec<RemoteParamWrite>,
    pub clear: Vec<String>,
}

fn provider_cache() -> &'static Mutex<Option<Vec<ProviderInfo>>> {
    static CACHE: OnceLock<Mutex<Option<Vec<ProviderInfo>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

pub fn providers() -> Result<Vec<ProviderInfo>, String> {
    if let Ok(guard) = provider_cache().lock() {
        if let Some(hit) = guard.as_ref() {
            return Ok(hit.clone());
        }
    }
    let raw = rclone_run(
        &["config".into(), "providers".into()],
        Duration::from_secs(40),
    )?;
    let parsed: Value = serde_json::from_str(json_body(&raw)).map_err(|_| {
        "This rclone is too old to list backends. LiClone needs rclone 1.65 or newer.".to_string()
    })?;
    let list = parsed
        .as_array()
        .ok_or("rclone config providers did not return a list.")?;
    let out = list.iter().filter_map(parse_provider).collect::<Vec<_>>();
    if let Ok(mut guard) = provider_cache().lock() {
        *guard = Some(out.clone());
    }
    Ok(out)
}

pub fn dump_remotes() -> Result<Vec<RemoteConfig>, String> {
    let raw = rclone_run(&["config".into(), "dump".into()], Duration::from_secs(20))?;
    let parsed: Value = serde_json::from_str(json_body(&raw)).map_err(|_| {
        "Could not read rclone’s config. LiClone needs rclone 1.65 or newer.".to_string()
    })?;
    let obj = parsed
        .as_object()
        .ok_or("rclone config dump did not return an object.")?;
    let mut out = Vec::new();
    for (name, body) in obj {
        let Some(body) = body.as_object() else {
            continue;
        };
        let kind = body
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let mut values = HashMap::new();
        for (key, value) in body {
            if key == "type" {
                continue;
            }
            values.insert(key.clone(), json_string(value));
        }
        out.push(RemoteConfig {
            name: name.clone(),
            kind,
            values,
        });
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

pub fn save_remote(body: RemoteWrite) -> Result<String, String> {
    let name = check_name(&body.name)?;
    let kind = body.kind.trim();
    if kind.is_empty() {
        return Err("Pick a remote type.".into());
    }
    let rclone = find_rclone().ok_or("rclone was not found on PATH.")?;
    let mut params = Vec::new();
    for param in &body.params {
        let key = param.key.trim();
        if key.is_empty() || key == "type" {
            continue;
        }
        let mut value = param.value.clone();
        if param.obscure {
            value = wizard::obscure(&rclone, value.trim())?;
        }
        params.push(RemoteParam {
            key: key.to_string(),
            value,
        });
    }
    let previous = body.previous_name.trim().trim_end_matches(':').to_string();
    let type_changed = !body.create && !body.previous_kind.is_empty() && body.previous_kind != kind;
    if body.create || type_changed {
        if type_changed && !previous.is_empty() {
            delete_remote(previous.clone())?;
        }
        let mut args = vec![
            "config".to_string(),
            "create".into(),
            name.clone(),
            kind.to_string(),
        ];
        for param in &params {
            args.push(param.key.clone());
            args.push(param.value.clone());
        }
        rclone_run(&args, Duration::from_secs(120))?;
        return Ok(format!("{name}:"));
    }
    if !previous.is_empty() && previous != name {
        rclone_run(
            &[
                "config".into(),
                "copy".into(),
                previous.clone(),
                name.clone(),
            ],
            Duration::from_secs(20),
        )?;
        delete_remote(previous)?;
    }
    for key in &body.clear {
        let key = key.trim();
        if key.is_empty() || key == "type" {
            continue;
        }
        rclone_confirm(
            &[
                "config".into(),
                "update".into(),
                name.clone(),
                key.to_string(),
                String::new(),
            ],
            Duration::from_secs(20),
        )?;
    }
    if !params.is_empty() {
        let mut args = vec!["config".to_string(), "update".into(), name.clone()];
        for param in &params {
            args.push(param.key.clone());
            args.push(param.value.clone());
        }
        rclone_confirm(&args, Duration::from_secs(40))?;
    }
    Ok(format!("{name}:"))
}

pub fn delete_remote(name: String) -> Result<(), String> {
    let name = check_name(&name)?;
    rclone_confirm(
        &["config".into(), "delete".into(), name],
        Duration::from_secs(20),
    )?;
    Ok(())
}

pub fn reconnect_remote(name: String) -> Result<String, String> {
    let name = check_name(&name)?;
    rclone_run(
        &["config".into(), "reconnect".into(), format!("{name}:")],
        Duration::from_secs(120),
    )?;
    Ok(format!("{name}:"))
}

fn check_name(name: &str) -> Result<String, String> {
    let name = name.trim().trim_end_matches(':').to_string();
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err("Use a remote name with letters, numbers, dashes, or underscores.".into());
    }
    Ok(name)
}

fn rclone_confirm(args: &[String], timeout: Duration) -> Result<String, String> {
    let mut flagged = Vec::with_capacity(args.len() + 1);
    flagged.push("--auto-confirm".to_string());
    flagged.extend(args.iter().cloned());
    match rclone_run(&flagged, timeout) {
        Err(err) if err.to_ascii_lowercase().contains("unknown flag") => rclone_run(args, timeout),
        other => other,
    }
}

fn json_body(raw: &str) -> &str {
    let bytes = raw.as_bytes();
    let start = bytes.iter().position(|b| *b == b'{' || *b == b'[').unwrap_or(0);
    raw[start..].trim()
}

fn rclone_run(args: &[String], timeout: Duration) -> Result<String, String> {
    let rclone = find_rclone().ok_or("rclone was not found on PATH.")?;
    let mut cmd = hidden_command(rclone.to_str().unwrap_or("rclone"));
    cmd.args(args);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let started = std::time::Instant::now();
    loop {
        if started.elapsed() > timeout {
            let _ = child.kill();
            return Err("rclone took too long. If a browser window opened, finish signing in and try again.".into());
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
                    return Ok(out);
                }
                let detail = if err.trim().is_empty() { out.trim() } else { err.trim() };
                return Err(if detail.is_empty() {
                    "rclone config command failed.".into()
                } else {
                    detail.to_string()
                });
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(200)),
            Err(e) => return Err(e.to_string()),
        }
    }
}

fn parse_provider(value: &Value) -> Option<ProviderInfo> {
    let obj = value.as_object()?;
    let name = text_at(obj, &["Name", "name"]);
    if name.is_empty() {
        return None;
    }
    let options = obj
        .get("Options")
        .or_else(|| obj.get("options"))
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(parse_option).collect())
        .unwrap_or_default();
    Some(ProviderInfo {
        name,
        description: clip(&text_at(obj, &["Description", "description"]), 400),
        options,
    })
}

fn parse_option(value: &Value) -> Option<ProviderOption> {
    let obj = value.as_object()?;
    let name = text_at(obj, &["Name", "name"]);
    if name.is_empty() {
        return None;
    }
    let examples = obj
        .get("Examples")
        .or_else(|| obj.get("examples"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let row = item.as_object()?;
                    let value = text_at(row, &["Value", "value"]);
                    if value.is_empty() {
                        return None;
                    }
                    Some(ProviderExample {
                        value,
                        help: clip(&text_at(row, &["Help", "help"]), 180),
                    })
                })
                .take(24)
                .collect()
        })
        .unwrap_or_default();
    let hide = obj
        .get("Hide")
        .or_else(|| obj.get("hide"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    Some(ProviderOption {
        name,
        help: clip(&text_at(obj, &["Help", "help"]), 700),
        default_value: {
            let explicit = text_at(obj, &["DefaultStr", "defaultStr"]);
            if explicit.is_empty() {
                json_string(obj.get("Default").or_else(|| obj.get("default")).unwrap_or(&Value::Null))
            } else {
                explicit
            }
        },
        required: bool_at(obj, &["Required", "required"]),
        password: bool_at(obj, &["IsPassword", "isPassword"]),
        sensitive: bool_at(obj, &["Sensitive", "sensitive"]) || bool_at(obj, &["IsPassword", "isPassword"]),
        advanced: bool_at(obj, &["Advanced", "advanced"]) || hide != 0,
        type_name: text_at(obj, &["Type", "type"]),
        examples,
    })
}

fn text_at(obj: &Map<String, Value>, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| obj.get(*key))
        .map(json_string)
        .unwrap_or_default()
}

fn bool_at(obj: &Map<String, Value>, keys: &[&str]) -> bool {
    keys.iter().find_map(|key| obj.get(*key)).map(|v| match v {
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_u64().unwrap_or(0) != 0,
        Value::String(s) => matches!(s.as_str(), "true" | "1" | "yes"),
        _ => false,
    }).unwrap_or(false)
}

fn json_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

fn clip(text: &str, max: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max {
        return trimmed.to_string();
    }
    let mut out = trimmed.chars().take(max).collect::<String>();
    out.push('…');
    out
}
