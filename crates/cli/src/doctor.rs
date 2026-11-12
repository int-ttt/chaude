//! `chaude doctor` and `/doctor`: the checks live in chaude_core::doctor. This file keeps the CLI entry point.

pub use chaude_core::doctor::{Check, Status, render};

/// `chaude doctor`: print the checks; exit 1 when any check fails.
pub fn run() -> i32 {
    let checks = checks_now();
    for line in render(&checks) {
        println!("{line}");
    }
    if checks.iter().any(|c| c.status == Status::Fail) {
        1
    } else {
        0
    }
}

pub fn checks_now() -> Vec<Check> {
    chaude_core::doctor::checks_now()
}
