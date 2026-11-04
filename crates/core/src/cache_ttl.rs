//! Prompt cache TTL choice for the child session (Go repl.go parseCacheTTL, core.go withCacheTTL and
//! observedTTL). Pure helpers: the flag is validated once, the child env pins the TTL, and the TTL seen in an
//! API response is read back from usage.cache_creation.

use serde_json::Value;

const TTL_KEY: &str = "CLAUDE_CODE_PROMPT_CACHE_TTL=";

// Go parseCacheTTL: --cache-ttl accepts exactly auto, 1h or 5m (case-sensitive).
pub fn parse_cache_ttl(s: &str) -> Result<String, String> {
    match s {
        "auto" | "1h" | "5m" => Ok(s.to_string()),
        _ => Err(format!("--cache-ttl must be auto, 1h or 5m (got {s:?})")),
    }
}

// Go withCacheTTL: "1h" or "5m" replace any CLAUDE_CODE_PROMPT_CACHE_TTL entry and append the new one last;
// "" and "auto" return the env unchanged.
pub fn with_cache_ttl(env: &[String], ttl: &str) -> Vec<String> {
    if ttl != "1h" && ttl != "5m" {
        return env.to_vec();
    }
    let mut out: Vec<String> = env
        .iter()
        .filter(|kv| !kv.starts_with(TTL_KEY))
        .cloned()
        .collect();
    out.push(format!("{TTL_KEY}{ttl}"));
    out
}

// Go observedTTL: "1h" or "5m" when one API call's usage.cache_creation wrote that tier; prev otherwise.
pub fn observed_ttl(usage: &Value, prev: &str) -> String {
    let cc = &usage["cache_creation"];
    let tokens = |key: &str| cc[key].as_f64().unwrap_or(0.0);
    if tokens("ephemeral_1h_input_tokens") > 0.0 {
        return "1h".to_string();
    }
    if tokens("ephemeral_5m_input_tokens") > 0.0 {
        return "5m".to_string();
    }
    prev.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Go TestParseCacheTTL
    #[test]
    fn parse_cache_ttl_accepts_three_values_only() {
        for s in ["auto", "1h", "5m"] {
            assert_eq!(parse_cache_ttl(s).as_deref(), Ok(s));
        }
        for s in ["", "10m", "1H", "AUTO"] {
            assert!(parse_cache_ttl(s).is_err(), "{s:?} must be rejected");
        }
    }

    // Go TestObservedTTL
    #[test]
    fn observed_ttl_reads_written_tier_else_prev() {
        let u1h = json!({"cache_creation": {"ephemeral_1h_input_tokens": 7581.0, "ephemeral_5m_input_tokens": 0.0}});
        let u5m = json!({"cache_creation": {"ephemeral_1h_input_tokens": 0.0, "ephemeral_5m_input_tokens": 120.0}});
        assert_eq!(observed_ttl(&u1h, "5m"), "1h");
        assert_eq!(observed_ttl(&u5m, "1h"), "5m");
        assert_eq!(observed_ttl(&json!({"input_tokens": 2.0}), "auto"), "auto");
    }

    // Go TestWithCacheTTL
    #[test]
    fn with_cache_ttl_pins_single_entry_last() {
        let base: Vec<String> = vec!["A=1".into(), format!("{TTL_KEY}5m")];
        for ttl in ["1h", "5m"] {
            let got = with_cache_ttl(&base, ttl);
            let n = got.iter().filter(|kv| kv.starts_with(TTL_KEY)).count();
            assert_eq!(n, 1, "{got:?}");
            assert_eq!(
                got.last().map(String::as_str),
                Some(format!("{TTL_KEY}{ttl}").as_str())
            );
        }
        for ttl in ["", "auto"] {
            assert_eq!(
                with_cache_ttl(&base, ttl),
                base,
                "ttl {ttl:?} must leave env as is"
            );
        }
    }
}
