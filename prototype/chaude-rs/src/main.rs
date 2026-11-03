// chaude-rs: inline (no alternate screen) prototype of the chaude stream-json loop.
// Usage:
//   chaude-rs [--model M] [--resume ID] [--claude-bin PATH]   interactive: type a line, get the answer
//   chaude-rs --parse-only [FILE]                             parse stream-json lines only, print counts

mod event;
mod session;

use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader, Write};
use std::process::ExitCode;
use std::sync::mpsc;

use event::{Block, Content, Delta, Event, StreamInner, parse_line, type_name};
use session::{Msg, Opts, Session};

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if argv.iter().any(|a| a == "-h" || a == "--help") {
        println!("usage: chaude-rs [--model M] [--resume ID] [--claude-bin PATH] | --parse-only [FILE]");
        return ExitCode::SUCCESS;
    }
    if let Some(i) = argv.iter().position(|a| a == "--parse-only") {
        return parse_only(argv.get(i + 1).map(String::as_str));
    }
    let opts = match parse_opts(&argv) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("chaude-rs: {e}");
            return ExitCode::from(2);
        }
    };
    run(opts)
}

fn parse_opts(argv: &[String]) -> Result<Opts, String> {
    let mut o = Opts { claude_bin: "claude".into(), model: "opus".into(), resume: None };
    let mut it = argv.iter();
    while let Some(a) = it.next() {
        let mut val = || it.next().cloned().ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--model" => o.model = val()?,
            "--resume" => o.resume = Some(val()?),
            "--claude-bin" => o.claude_bin = val()?,
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(o)
}

// Reads stream-json lines from FILE (or stdin) and counts top-level types. Exit 1 on any parse error.
fn parse_only(path: Option<&str>) -> ExitCode {
    let reader: Box<dyn BufRead> = match path {
        Some(p) => match std::fs::File::open(p) {
            Ok(f) => Box::new(BufReader::new(f)),
            Err(e) => {
                eprintln!("chaude-rs: {p}: {e}");
                return ExitCode::from(2);
            }
        },
        None => Box::new(io::stdin().lock()),
    };
    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    let (mut lines, mut errors) = (0usize, 0usize);
    for line in reader.lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        lines += 1;
        match parse_line(&line) {
            Ok(ev) => *counts.entry(type_name(&ev)).or_insert(0) += 1,
            Err(_) => errors += 1,
        }
    }
    println!("lines={lines} parse_errors={errors}");
    for (k, v) in &counts {
        println!("  {k}: {v}");
    }
    if errors == 0 { ExitCode::SUCCESS } else { ExitCode::from(1) }
}

fn run(opts: Opts) -> ExitCode {
    let (tx, rx) = mpsc::channel::<Msg>();
    session::spawn_prompt_reader(tx.clone());
    let mut sess: Session = match session::spawn(&opts, tx.clone()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("chaude-rs: cannot start {}: {e}", opts.claude_bin);
            return ExitCode::from(1);
        }
    };
    drop(tx);

    let mut out = Renderer { streamed: false };
    print_prompt();
    for msg in rx {
        match msg {
            Msg::Prompt(text) => {
                let Some(stdin) = sess.stdin.as_mut() else { continue };
                if let Err(e) = session::write_user(stdin, &text) {
                    eprintln!("\nchaude-rs: write to claude failed: {e}");
                    break;
                }
            }
            Msg::StdinEof => sess.stdin = None,
            Msg::Line(line) => match parse_line(&line) {
                Ok(ev) => out.handle(&ev, &mut sess),
                Err(_) => eprintln!("[unparsed] {line}"),
            },
            Msg::ChildEof => break,
        }
    }
    let _ = io::stdout().flush();
    match sess.child.wait() {
        Ok(st) if st.success() => ExitCode::SUCCESS,
        Ok(st) => ExitCode::from(st.code().unwrap_or(1).clamp(1, 255) as u8),
        Err(e) => {
            eprintln!("chaude-rs: wait failed: {e}");
            ExitCode::from(1)
        }
    }
}

fn print_prompt() {
    print!("> ");
    let _ = io::stdout().flush();
}

struct Renderer {
    // true once text_delta chunks printed for the current turn; the full assistant text is then skipped.
    streamed: bool,
}

impl Renderer {
    fn handle(&mut self, ev: &Event, sess: &mut Session) {
        match ev {
            Event::StreamEvent(w) if is_main(w.parent_tool_use_id.as_deref()) => {
                if let StreamInner::ContentBlockDelta { delta: Delta::TextDelta { text } } = &w.event {
                    print!("{text}");
                    let _ = io::stdout().flush();
                    self.streamed = true;
                }
            }
            Event::Assistant(env) => {
                let Some(Content::Blocks(blocks)) = &env.message.content else { return };
                for b in blocks {
                    match b {
                        Block::Text { text } if !self.streamed => println!("{text}"),
                        Block::ToolUse { name, input, .. } => {
                            let summary = input.as_ref().map(|v| truncate(&v.to_string(), 160)).unwrap_or_default();
                            println!("\n[tool] {name} {summary}");
                        }
                        _ => {}
                    }
                }
            }
            Event::ControlRequest(c) if c.request.subtype == "can_use_tool" => {
                // No interactive prompt in this prototype: deny and say so inline.
                let tool = c.request.tool_name.as_deref().unwrap_or("?");
                println!("\n[permission] {tool} denied (prototype has no permission prompt)");
                let input = c.request.input.clone().unwrap_or(serde_json::json!({}));
                if let Some(stdin) = sess.stdin.as_mut() {
                    let _ = session::write_permission(stdin, &c.request_id, false, input, "denied by chaude-rs prototype");
                }
            }
            Event::Result(r) => {
                if r.is_error == Some(true) {
                    println!("\n[error] {}", r.result.as_deref().unwrap_or(r.subtype.as_deref().unwrap_or("error")));
                }
                println!();
                self.streamed = false;
                print_prompt();
            }
            _ => {}
        }
    }
}

// Subagent events carry a parent_tool_use_id; only the main thread is rendered.
fn is_main(parent: Option<&str>) -> bool {
    parent.is_none_or(|p| p.is_empty())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut t: String = s.chars().take(max).collect();
    t.push_str("...");
    t
}
