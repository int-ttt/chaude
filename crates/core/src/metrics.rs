//! Per-turn metrics log (Go metrics.go): one JSON object per line in `<dir>/<YYYY-MM-DD>.jsonl`, UTC date.
//! `dir == None` disables it; every call is then a no-op.

use serde_json::{Map, Value, json};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Metrics {
    dir: Option<PathBuf>,
    flags: Value,
    // Serializes appends from concurrent writers (Go mu).
    lock: Mutex<()>,
}

impl Metrics {
    pub fn new(dir: Option<PathBuf>, flags: Value) -> Self {
        Metrics {
            dir,
            flags,
            lock: Mutex::new(()),
        }
    }

    // Adds "ts" (RFC3339 UTC) and "flags" to a copy of `rec` and appends it as one line.
    pub fn log(&self, rec: Value) -> io::Result<()> {
        let Some(dir) = &self.dir else {
            return Ok(());
        };
        let secs = unix_secs();
        let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
        let mut out = match rec {
            Value::Object(map) => map,
            _ => Map::new(),
        };
        out.insert("ts".into(), Value::String(rfc3339(secs)));
        out.insert("flags".into(), self.flags.clone());

        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        fs::create_dir_all(dir)?;
        let path = dir.join(format!("{y:04}-{m:02}-{d:02}.jsonl"));
        let mut f = OpenOptions::new().create(true).append(true).open(path)?;
        writeln!(f, "{}", Value::Object(out))
    }
}

// Token counts of one API usage object (Go usageFields); ctx is the context-size proxy (Go ctxTokens).
pub fn usage_fields(usage: &Value) -> Value {
    let input = num(usage.get("input_tokens"));
    let cache_read = num(usage.get("cache_read_input_tokens"));
    let cache_creation = num(usage.get("cache_creation_input_tokens"));
    json!({
        "input_tokens": input,
        "cache_read_input_tokens": cache_read,
        "cache_creation_input_tokens": cache_creation,
        "output_tokens": num(usage.get("output_tokens")),
        "ctx": input + cache_read + cache_creation,
    })
}

// Go num: a JSON number as an integer (truncated); anything else is 0.
pub(crate) fn num(v: Option<&Value>) -> i64 {
    v.and_then(Value::as_f64).map_or(0, |f| f as i64)
}

fn unix_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// Unix seconds (UTC) as "YYYY-MM-DDTHH:MM:SSZ", the form Go time.RFC3339 gives for UTC.
pub(crate) fn rfc3339(secs: i64) -> String {
    let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
    let rem = secs.rem_euclid(86_400);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

// Today as YYMMDD (UTC): the date prefix of notes/runs result files (Go agentResultRel).
pub(crate) fn yymmdd_today() -> String {
    let (y, m, d) = civil_from_days(unix_secs().div_euclid(86_400));
    format!("{:02}{m:02}{d:02}", y.rem_euclid(100))
}

// Days since 1970-01-01 to (year, month, day). Howard Hinnant's civil_from_days.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    #[test]
    fn usage_fields_sums_ctx_and_defaults_missing_to_zero() {
        let usage = json!({"input_tokens": 3, "cache_read_input_tokens": 10,
            "cache_creation_input_tokens": 4, "output_tokens": 7});
        assert_eq!(
            usage_fields(&usage),
            json!({"input_tokens": 3, "cache_read_input_tokens": 10,
                "cache_creation_input_tokens": 4, "output_tokens": 7, "ctx": 17})
        );
        assert_eq!(
            usage_fields(&json!({"output_tokens": 2.9})),
            json!({"input_tokens": 0, "cache_read_input_tokens": 0,
                "cache_creation_input_tokens": 0, "output_tokens": 2, "ctx": 0})
        );
        assert_eq!(usage_fields(&Value::Null)["ctx"], 0);
    }

    // Port of Go TestMetricsLog.
    #[test]
    fn log_appends_one_line_per_record_with_ts_and_flags() {
        let tmp = TempDir::new("metrics-log");
        let m = Metrics::new(Some(tmp.path().to_path_buf()), json!({"lean": true}));
        for i in 0..2 {
            m.log(json!({"event": "turn", "n": i})).unwrap();
        }
        let files: Vec<_> = fs::read_dir(tmp.path()).unwrap().collect();
        assert_eq!(files.len(), 1);
        let path = files[0].as_ref().unwrap().path();
        let text = fs::read_to_string(path).unwrap();
        let lines: Vec<&str> = text.trim_end_matches('\n').split('\n').collect();
        assert_eq!(lines.len(), 2);
        for line in lines {
            let rec: Value = serde_json::from_str(line).unwrap();
            assert!(rec["ts"].is_string(), "missing ts in {line}");
            assert_eq!(rec["flags"], json!({"lean": true}));
        }
    }

    // Port of Go TestMetricsNilAndDisabled: a disabled log writes nothing.
    #[test]
    fn disabled_log_is_noop() {
        let tmp = TempDir::new("metrics-off");
        let m = Metrics::new(None, Value::Null);
        m.log(json!({"a": 1})).unwrap();
        assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 0);
    }

    #[test]
    fn civil_and_rfc3339_match_known_instants() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(951_782_400 + 3661), "2000-02-29T01:01:01Z");
    }
}
