use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BandwidthRule {
    pub start: String,
    pub limit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BandwidthSchedule {
    pub enabled: bool,
    pub rules: Vec<BandwidthRule>,
}

impl Default for BandwidthSchedule {
    fn default() -> Self {
        Self {
            enabled: false,
            rules: vec![
                BandwidthRule {
                    start: "08:00".into(),
                    limit: "2M".into(),
                },
                BandwidthRule {
                    start: "22:00".into(),
                    limit: "off".into(),
                },
            ],
        }
    }
}

fn path() -> Result<PathBuf, String> {
    let base = dirs::data_dir().ok_or("Could not resolve app data directory.")?;
    let dir = base.join("liclone");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("bandwidth.json"))
}

pub fn load() -> BandwidthSchedule {
    let Ok(path) = path() else {
        return BandwidthSchedule::default();
    };
    let Ok(raw) = fs::read_to_string(path) else {
        return BandwidthSchedule::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn save(schedule: BandwidthSchedule) -> Result<BandwidthSchedule, String> {
    let json = serde_json::to_string_pretty(&schedule).map_err(|e| e.to_string())?;
    fs::write(path()?, json).map_err(|e| e.to_string())?;
    Ok(schedule)
}

pub fn timetable(schedule: &BandwidthSchedule) -> Option<String> {
    if !schedule.enabled {
        return None;
    }
    let parts: Vec<String> = schedule
        .rules
        .iter()
        .filter(|r| !r.start.trim().is_empty() && !r.limit.trim().is_empty())
        .map(|r| format!("{},{}", r.start.trim(), r.limit.trim()))
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

pub fn append_bwlimit(argv: &mut Vec<String>) {
    let Some(limit) = timetable(&load()) else {
        return;
    };
    if argv.iter().any(|a| a == "--bwlimit") {
        return;
    }
    argv.push("--bwlimit".into());
    argv.push(limit);
}

pub fn current_rate(schedule: &BandwidthSchedule) -> Option<String> {
    let mut parsed: Vec<(u32, String)> = schedule
        .rules
        .iter()
        .filter_map(|r| {
            let mins = parse_hhmm(r.start.trim())?;
            let limit = r.limit.trim();
            if limit.is_empty() {
                None
            } else {
                Some((mins, limit.to_string()))
            }
        })
        .collect();
    if parsed.is_empty() {
        return None;
    }
    parsed.sort_by_key(|row| row.0);
    let now = local_minutes();
    let mut chosen = parsed.last().map(|(_, limit)| limit.clone())?;
    for (mins, limit) in &parsed {
        if *mins <= now {
            chosen = limit.clone();
        }
    }
    Some(chosen)
}

fn local_minutes() -> u32 {
    #[cfg(windows)]
    {
        #[repr(C)]
        struct SystemTime {
            _year: u16,
            _month: u16,
            _dow: u16,
            _day: u16,
            hour: u16,
            minute: u16,
            _second: u16,
            _ms: u16,
        }
        #[link(name = "kernel32")]
        extern "system" {
            fn GetLocalTime(time: *mut SystemTime);
        }
        let mut time = SystemTime {
            _year: 0,
            _month: 0,
            _dow: 0,
            _day: 0,
            hour: 0,
            minute: 0,
            _second: 0,
            _ms: 0,
        };
        unsafe { GetLocalTime(&mut time) };
        return u32::from(time.hour) * 60 + u32::from(time.minute);
    }
    #[cfg(not(windows))]
    {
        0
    }
}

fn parse_hhmm(raw: &str) -> Option<u32> {
    let (h, m) = raw.split_once(':')?;
    let h: u32 = h.parse().ok()?;
    let m: u32 = m.parse().ok()?;
    if h > 23 || m > 59 {
        return None;
    }
    Some(h * 60 + m)
}
