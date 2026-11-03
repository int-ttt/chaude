//! Argument parsing shared by the scripts and the settings generator (Go: internal/lean).
//! Like Go's flag.FlagSet.Parse + ParseInterspersed: flags may appear before or after positionals.

use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    /// -h / --help was given; the caller prints usage.
    Help,
    /// Unknown flag, or a value flag without its value.
    Msg(String),
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Parsed {
    /// Last value given per value flag, keyed by the flag name without dashes.
    pub values: BTreeMap<String, String>,
    /// Non-flag arguments in order.
    pub positionals: Vec<String>,
}

/// Parses `args`. Every name in `value_flags` takes one value: `-x v`, `--x v` or `--x=v`.
/// `--` ends flag parsing; a lone `-` is a positional.
pub fn parse_interspersed(args: &[String], value_flags: &[&str]) -> Result<Parsed, ParseError> {
    let mut out = Parsed::default();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        i += 1;
        if arg == "--" {
            out.positionals.extend(args[i..].iter().cloned());
            break;
        }
        let Some(body) = arg.strip_prefix('-').filter(|b| !b.is_empty()) else {
            out.positionals.push(arg.clone());
            continue;
        };
        let body = body.strip_prefix('-').unwrap_or(body);
        if body == "h" || body == "help" {
            return Err(ParseError::Help);
        }
        let (name, inline) = match body.split_once('=') {
            Some((n, v)) => (n, Some(v.to_string())),
            None => (body, None),
        };
        if !value_flags.contains(&name) {
            return Err(ParseError::Msg(format!(
                "flag provided but not defined: -{name}"
            )));
        }
        let value = match inline {
            Some(v) => v,
            None => {
                let Some(v) = args.get(i) else {
                    return Err(ParseError::Msg(format!("flag needs an argument: -{name}")));
                };
                i += 1;
                v.clone()
            }
        };
        out.values.insert(name.to_string(), value);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn flags_and_positionals_interleave() {
        let p = parse_interspersed(
            &s(&["mac", "--out", "a.json", "--src=b.json", "extra"]),
            &["out", "src"],
        )
        .unwrap();
        assert_eq!(p.positionals, s(&["mac", "extra"]));
        assert_eq!(p.values.get("out").map(String::as_str), Some("a.json"));
        assert_eq!(p.values.get("src").map(String::as_str), Some("b.json"));
    }

    #[test]
    fn double_dash_ends_flags_and_errors_are_reported() {
        let p = parse_interspersed(&s(&["--", "--out", "x"]), &["out"]).unwrap();
        assert_eq!(p.positionals, s(&["--out", "x"]));
        assert!(p.values.is_empty());
        assert_eq!(
            parse_interspersed(&s(&["-h"]), &["out"]),
            Err(ParseError::Help)
        );
        assert_eq!(
            parse_interspersed(&s(&["-bogus"]), &["out"]),
            Err(ParseError::Msg(
                "flag provided but not defined: -bogus".into()
            ))
        );
        assert_eq!(
            parse_interspersed(&s(&["--out"]), &["out"]),
            Err(ParseError::Msg("flag needs an argument: -out".into()))
        );
    }
}
