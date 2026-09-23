use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RcProbe {
    pub addr: String,
    pub ok: bool,
    pub error: Option<String>,
    pub stats: Option<Value>,
    pub vfs: Option<Value>,
    pub transferring: Vec<TransferItem>,
    pub transferred: Vec<Value>,
    pub options: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TransferItem {
    pub name: String,
    pub size: i64,
    pub bytes: i64,
    pub percentage: f64,
    pub speed: f64,
    pub speed_avg: f64,
    pub eta: Option<i64>,
    pub group: Option<String>,
    pub src_fs: Option<String>,
    pub dst_fs: Option<String>,
    #[serde(default)]
    pub phase: Option<String>,
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(800))
        .no_proxy()
        .build()
        .expect("http client")
}

fn url(addr: &str, path: &str) -> String {
    let addr = addr.trim().trim_end_matches('/');
    if addr.starts_with("http://") || addr.starts_with("https://") {
        format!("{addr}{path}")
    } else {
        format!("http://{addr}{path}")
    }
}

async fn rc_post(
    addr: &str,
    path: &str,
    user: Option<&str>,
    pass: Option<&str>,
    body: Value,
) -> Result<Value, String> {
    let mut req = client().post(url(addr, path)).json(&body);
    if let Some(u) = user {
        req = req.basic_auth(u, pass);
    }
    let res = req.send().await.map_err(|e| e.to_string())?;
    let status = res.status();
    let text = res.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("{status}: {text}"));
    }
    if text.trim().is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str(&text).map_err(|e| format!("invalid json from {path}: {e}; {text}"))
}

fn parse_transferring(stats: &Value) -> Vec<TransferItem> {
    let Some(arr) = stats.get("transferring").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    arr.iter()
        .map(|item| TransferItem {
            name: item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string(),
            size: item.get("size").and_then(|v| v.as_i64()).unwrap_or(-1),
            bytes: item.get("bytes").and_then(|v| v.as_i64()).unwrap_or(0),
            percentage: item
                .get("percentage")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            speed: item.get("speed").and_then(|v| v.as_f64()).unwrap_or(0.0),
            speed_avg: item.get("speedAvg").and_then(|v| v.as_f64()).unwrap_or(0.0),
            eta: item.get("eta").and_then(|v| v.as_i64()),
            group: item
                .get("group")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            src_fs: item
                .get("srcFs")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            dst_fs: item
                .get("dstFs")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            phase: Some("rc".into()),
        })
        .collect()
}

pub async fn probe(
    addr: String,
    user: Option<String>,
    pass: Option<String>,
) -> RcProbe {
    let u = user.as_deref();
    let p = pass.as_deref();
    let stats = match rc_post(&addr, "/core/stats", u, p, json!({})).await {
        Ok(v) => v,
        Err(e) => {
            return RcProbe {
                addr,
                ok: false,
                error: Some(e),
                stats: None,
                vfs: None,
                transferring: Vec::new(),
                transferred: Vec::new(),
                options: None,
            };
        }
    };
    let transferring = parse_transferring(&stats);
    let transferred = rc_post(&addr, "/core/transferred", u, p, json!({}))
        .await
        .ok()
        .and_then(|v| v.get("transferred").cloned())
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    let vfs = rc_post(&addr, "/vfs/stats", u, p, json!({})).await.ok();
    let options = rc_post(&addr, "/options/get", u, p, json!({})).await.ok();

    RcProbe {
        addr,
        ok: true,
        error: None,
        stats: Some(stats),
        vfs,
        transferring,
        transferred,
        options,
    }
}

pub async fn set_bwlimit(addr: &str, rate: &str) -> Result<(), String> {
    rc_post(addr, "/core/bwlimit", None, None, json!({ "rate": rate }))
        .await
        .map(|_| ())
}

pub async fn probe_many(targets: Vec<RcTarget>) -> Vec<RcProbe> {
    let mut out = Vec::with_capacity(targets.len());
    for t in targets {
        out.push(probe(t.addr, t.user, t.pass).await);
    }
    out
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RcTarget {
    pub addr: String,
    pub user: Option<String>,
    pub pass: Option<String>,
}
