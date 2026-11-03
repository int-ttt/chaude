//! Per-prompt USD ledger and cost estimates (Go harness budget.go, U9). cap <= 0 = off. In memory only.
//! total_cost_usd from the claude CLI is cumulative per process, so charge books the delta vs last[proc].

use std::collections::HashMap;

use crate::event::Usage;

#[derive(Debug, Clone)]
pub struct Budget {
    cap: f64,
    spent: f64,
    running: f64,
    last: HashMap<u64, f64>,
}

impl Budget {
    pub fn new(cap: f64) -> Self {
        Budget {
            cap,
            spent: 0.0,
            running: 0.0,
            last: HashMap::new(),
        }
    }

    pub fn cap(&self) -> f64 {
        self.cap
    }

    // Starts a new prompt: spent and running to 0; per-process baselines are kept.
    pub fn reset(&mut self) {
        self.spent = 0.0;
        self.running = 0.0;
    }

    // Books the cost growth of process `proc` given its cumulative total. A drop (new process or CLI
    // counter reset) books the whole total.
    pub fn charge(&mut self, proc: u64, total: f64) {
        if self.cap <= 0.0 {
            return;
        }
        let mut d = total - self.last.get(&proc).copied().unwrap_or(0.0);
        if d < 0.0 {
            d = total;
        }
        self.spent += d;
        self.last.insert(proc, total);
        self.running = 0.0;
    }

    // Books the cost of a separate one-shot process (Go chargeOneShot, /usage).
    pub fn charge_one_shot(&mut self, cost: f64) {
        if self.cap <= 0.0 || cost <= 0.0 {
            return;
        }
        self.spent += cost;
    }

    // Reports whether a call with estimated cost `est` may start.
    pub fn allow(&self, est: f64) -> bool {
        if self.cap <= 0.0 {
            return true;
        }
        !(self.spent >= self.cap || self.spent + est > self.cap)
    }

    // Reports whether spent plus the in-flight estimate reached the cap.
    pub fn over(&mut self, running: f64) -> bool {
        if self.cap <= 0.0 {
            return false;
        }
        self.running = running;
        self.spent + running >= self.cap
    }

    pub fn spent(&self) -> f64 {
        self.spent
    }
}

#[derive(Clone, Copy)]
struct ModelPrice {
    input: f64,
    write_5m: f64,
    write_1h: f64,
    read: f64,
    output: f64,
}

// USD per million tokens. Source: https://docs.claude.com/en/docs/about-claude/pricing
// (redirects to platform.claude.com), fetched 2026-10-29. Assumed alias map: opus=Opus 5.5,
// sonnet=Sonnet 5.5, haiku=Haiku 5.5 (up to 100k tokens).
fn price_table(alias: &str) -> Option<ModelPrice> {
    match alias {
        "opus" => Some(ModelPrice {
            input: 4.0,
            write_5m: 5.0,
            write_1h: 8.0,
            read: 0.20,
            output: 20.0,
        }),
        "sonnet" => Some(ModelPrice {
            input: 2.0,
            write_5m: 2.50,
            write_1h: 4.0,
            read: 0.10,
            output: 10.0,
        }),
        "haiku" => Some(ModelPrice {
            input: 0.10,
            write_5m: 0.125,
            write_1h: 0.20,
            read: 0.01,
            output: 0.50,
        }),
        _ => None,
    }
}

// Maps a full model id (claude-opus-5-5, claude-sonnet-5-5, ...) to its price alias. Other names are
// returned as-is, so they find no price.
fn price_key(model: &str) -> String {
    let m = model.to_lowercase();
    for alias in ["opus", "sonnet", "haiku"] {
        if m.contains(alias) {
            return alias.to_string();
        }
    }
    model.to_string()
}

// Planned cost of the next call: ctx tokens at the cache-read price (warm) or the cache-write price for
// ttl ("1h" 2x input, else 1.25x), plus 4k output tokens. Unknown model = 0.
pub fn estimate(model: &str, ctx: u64, warm: bool, ttl: &str) -> f64 {
    let Some(p) = price_table(&price_key(model)) else {
        return 0.0;
    };
    let per = if warm {
        p.read
    } else if ttl == "1h" {
        p.write_1h
    } else {
        p.write_5m
    };
    (ctx as f64 * per + 4000.0 * p.output) / 1e6
}

// In-flight cost guess from one main assistant usage event.
pub fn mid_turn_estimate(model: &str, u: &Usage) -> f64 {
    let Some(p) = price_table(&price_key(model)) else {
        return 0.0;
    };
    let n = |v: Option<u64>| v.unwrap_or(0) as f64;
    (n(u.cache_read_input_tokens) * p.read
        + n(u.cache_creation_input_tokens) * p.write_5m
        + n(u.input_tokens) * p.input
        + n(u.output_tokens) * p.output)
        / 1e6
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_per_process() {
        let mut b = Budget::new(1.0);
        b.charge(1, 0.10);
        b.charge(1, 0.25); // +0.15
        b.charge(2, 0.05); // other process starts at its own baseline
        assert!((b.spent() - 0.30).abs() < 1e-9, "spent {}", b.spent());
        b.charge(1, 0.01); // counter dropped: whole total is new spend
        assert!((b.spent() - 0.31).abs() < 1e-9, "after drop {}", b.spent());
    }

    #[test]
    fn cap_zero_is_off() {
        let mut b = Budget::new(0.0);
        b.charge_one_shot(5.0);
        b.charge(1, 5.0);
        assert_eq!(b.spent(), 0.0);
        assert!(b.allow(100.0));
        assert!(!b.over(100.0));
    }

    #[test]
    fn reset_clears_spent_and_keeps_baseline() {
        let mut b = Budget::new(1.0);
        b.charge(7, 0.9);
        b.reset();
        assert!(b.spent() == 0.0 && b.allow(0.5));
        b.charge(7, 1.0); // baseline kept: only +0.1
        assert!(
            (b.spent() - 0.1).abs() < 1e-9,
            "baseline lost {}",
            b.spent()
        );
    }

    #[test]
    fn resumed_process_books_whole_first_total() {
        // A new process (new proc id) is never seeded from a prior session: its whole first total is spend.
        let mut b = Budget::new(1.0);
        b.charge(2, 0.2);
        assert_eq!(b.spent(), 0.2);
    }

    #[test]
    fn allow_and_over_against_cap() {
        let mut b = Budget::new(0.1);
        b.charge_one_shot(0.02);
        assert!(b.allow(0.05));
        assert!(!b.allow(0.09));
        assert!(!b.over(0.07)); // 0.02 + 0.07 < 0.1
        assert!(b.over(0.08)); // 0.02 + 0.08 >= 0.1
    }

    #[test]
    fn full_model_id_is_priced_like_alias() {
        for (alias, id) in [("opus", "claude-opus-5-5"), ("sonnet", "claude-sonnet-5-5")] {
            let a = estimate(alias, 1000, true, "1h");
            let f = estimate(id, 1000, true, "1h");
            assert!(a > 0.0 && a == f, "estimate {alias}={a} {id}={f}");
        }
        let u = Usage {
            input_tokens: Some(1000),
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
            output_tokens: Some(100),
        };
        assert!(
            mid_turn_estimate("claude-opus-5-5", &u) > 0.0,
            "full id not priced"
        );
    }

    #[test]
    fn unknown_model_costs_nothing() {
        assert_eq!(estimate("gpt-x", 1000, true, ""), 0.0);
    }
}
