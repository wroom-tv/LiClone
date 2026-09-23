use std::collections::HashMap;

use serde::Serialize;

const VALUE_FLAGS: &[&str] = &[
    "--rc-addr",
    "--rc-user",
    "--rc-pass",
    "--rc-htpasswd",
    "--rc-cert",
    "--rc-key",
    "--rc-client-ca",
    "--rc-job-expire-duration",
    "--rc-job-expire-interval",
    "--cache-dir",
    "--vfs-cache-mode",
    "--vfs-cache-max-size",
    "--vfs-cache-max-age",
    "--vfs-cache-poll-interval",
    "--vfs-write-back",
    "--vfs-read-ahead",
    "--vfs-read-chunk-size",
    "--vfs-read-chunk-size-limit",
    "--vfs-disk-space-total-size",
    "--dir-cache-time",
    "--poll-interval",
    "--buffer-size",
    "--bwlimit",
    "--bwlimit-file",
    "--config",
    "--log-file",
    "--log-level",
    "--log-format",
    "--stats",
    "--stats-file-name-length",
    "--volname",
    "--uid",
    "--gid",
    "--umask",
    "--attr-timeout",
    "--timeout",
    "--contimeout",
    "--retries",
    "--retries-sleep",
    "--low-level-retries",
    "--bind",
    "--user-agent",
    "--multi-thread-cutoff",
    "--multi-thread-streams",
    "--transfers",
    "--checkers",
    "--max-depth",
    "--sfn-hash-type",
    "--vfs-cache-min-free-space",
    "--tpslimit",
    "--tpslimit-burst",
    "--file-perms",
    "--dir-perms",
    "--drive-chunk-size",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedRclone {
    pub binary: String,
    pub verb: String,
    pub remote: Option<String>,
    pub mount_point: Option<String>,
    pub flags: HashMap<String, Option<String>>,
    pub positional: Vec<String>,
    pub raw: Vec<String>,
}

/// Split a Windows process CommandLine into argv (CommandLineToArgvW rules).
pub fn split_cmdline(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if c == '"' {
            in_quotes = !in_quotes;
            i += 1;
            continue;
        }
        if !in_quotes && c.is_whitespace() {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            i += 1;
            continue;
        }
        cur.push(c);
        i += 1;
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

pub fn normalize_argv(argv: &[String]) -> Vec<String> {
    if argv.is_empty() {
        return Vec::new();
    }
    if argv.len() == 1 && argv[0].contains(char::is_whitespace) {
        return split_cmdline(&argv[0]);
    }
    argv.to_vec()
}

pub fn parse_argv(argv: &[String]) -> ParsedRclone {
    let argv = normalize_argv(argv);
    let binary = argv.first().cloned().unwrap_or_else(|| "rclone".into());
    let mut flags: HashMap<String, Option<String>> = HashMap::new();
    let mut positional = Vec::new();
    let mut i = 1usize;
    let value_flags: std::collections::HashSet<&str> = VALUE_FLAGS.iter().copied().collect();

    while i < argv.len() {
        let a = &argv[i];
        if a == "--" {
            positional.extend(argv[i + 1..].iter().cloned());
            break;
        }
        if let Some(stripped) = a.strip_prefix("--") {
            if let Some((k, v)) = stripped.split_once('=') {
                flags.insert(format!("--{k}"), Some(v.to_string()));
                i += 1;
                continue;
            }
            let flag = if a.starts_with("--") {
                a.clone()
            } else {
                format!("--{stripped}")
            };
            let takes_value = value_flags.contains(flag.as_str());
            if takes_value
                && i + 1 < argv.len()
                && !argv[i + 1].starts_with('-')
            {
                flags.insert(flag, Some(argv[i + 1].clone()));
                i += 2;
                continue;
            }
            flags.insert(flag, None);
            i += 1;
            continue;
        }
        if a.starts_with('-') && a.len() > 1 {
            flags.insert(a.clone(), None);
            i += 1;
            continue;
        }
        positional.push(a.clone());
        i += 1;
    }

    let verb = positional
        .first()
        .cloned()
        .unwrap_or_else(|| "unknown".into());
    let remote = positional.get(1).cloned();
    let mount_point = if verb == "mount" {
        positional.get(2).cloned()
    } else {
        None
    };

    ParsedRclone {
        binary,
        verb,
        remote,
        mount_point,
        flags,
        positional,
        raw: argv,
    }
}

pub fn flag_value<'a>(parsed: &'a ParsedRclone, name: &str) -> Option<&'a str> {
    parsed
        .flags
        .get(name)
        .and_then(|v| v.as_deref())
}

pub fn has_flag(parsed: &ParsedRclone, name: &str) -> bool {
    parsed.flags.contains_key(name)
}

pub fn rc_addr(parsed: &ParsedRclone) -> Option<String> {
    let implied = parsed.verb == "rcd"
        || has_flag(parsed, "--rc")
        || has_flag(parsed, "--rc-web-gui")
        || flag_value(parsed, "--rc-addr").is_some();
    if implied {
        let raw = flag_value(parsed, "--rc-addr").unwrap_or("127.0.0.1:5572");
        Some(normalize_addr(raw))
    } else {
        None
    }
}

pub fn normalize_addr(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with(':') {
        format!("127.0.0.1{trimmed}")
    } else if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .trim_end_matches('/')
            .to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn quote_win(arg: &str) -> String {
    if arg.is_empty() {
        return "\"\"".into();
    }
    if !arg.contains([' ', '\t', '"', '\'', '&', '^', '%']) {
        return arg.to_string();
    }
    format!("\"{}\"", arg.replace('"', "\"\""))
}
