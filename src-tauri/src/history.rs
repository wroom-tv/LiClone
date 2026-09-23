use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEvent {
    pub at: i64,
    pub name: String,
    pub size: i64,
    pub status: String,
    pub detail: String,
}

fn path() -> Result<PathBuf, String> {
    let base = dirs::data_dir().ok_or("Could not resolve app data directory.")?;
    let dir = base.join("liclone");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("history.json"))
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn list() -> Vec<HistoryEvent> {
    let Ok(path) = path() else {
        return Vec::new();
    };
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

fn save(events: &[HistoryEvent]) -> Result<(), String> {
    let json = serde_json::to_string_pretty(events).map_err(|e| e.to_string())?;
    fs::write(path()?, json).map_err(|e| e.to_string())
}

fn push(event: HistoryEvent) {
    let mut events = list();
    if events.iter().any(|e| {
        e.status == event.status && e.name == event.name && e.detail == event.detail && now() - e.at < 120
    }) {
        return;
    }
    events.push(event);
    if events.len() > 200 {
        let skip = events.len() - 200;
        events.drain(0..skip);
    }
    let _ = save(&events);
}

pub fn push_finished(name: &str, size: i64) {
    push(HistoryEvent {
        at: now(),
        name: name.to_string(),
        size,
        status: "finished".into(),
        detail: "Uploaded. The file is no longer waiting on disk.".into(),
    });
}

pub fn note_failures(messages: &[String]) {
    for message in messages {
        let detail = message.trim();
        if detail.is_empty() {
            continue;
        }
        let name = detail
            .split(['\\', '/'])
            .last()
            .unwrap_or(detail)
            .to_string();
        push(HistoryEvent {
            at: now(),
            name,
            size: 0,
            status: "failed".into(),
            detail: detail.to_string(),
        });
    }
}
