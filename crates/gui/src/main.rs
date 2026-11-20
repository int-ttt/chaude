//! chaude GUI entry point. One native window (tao) with the embedded page (wry webview).
//! CHAUDE_GUI_QUIT_AFTER_SECS=<n> closes the window after n seconds (smoke runs only).
//! CHAUDE_GUI_SMOKE_START=chat|code:<folder> starts a session 1 second after launch (smoke runs only).
//! CHAUDE_GUI_SMOKE_SUBMIT=<text> sends that text as a submit command 2 seconds after start (smoke runs only).

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chaude_gui::bridge;
use chaude_gui::front::Front;
use chaude_gui::guard::{self, Claim};
use chaude_gui::page;
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use wry::WebViewBuilder;

// Smoke only: the page's DOM counts, read 2 seconds before the smoke quit (rows and assistant text characters).
const PROBE_JS: &str = "JSON.stringify({rows: document.querySelectorAll('#column > .row').length, assistant_chars: [...document.querySelectorAll('.row.assistant .md')].reduce((n, el) => n + el.textContent.length, 0)})";

// Events the window thread gets from other threads: the page's messages, a second launch's focus request, and a
// wake from the engine or a job (the page events are then pulled from Front).
#[derive(Debug)]
enum UserEvent {
    FromPage(String),
    Focus,
    Wake,
}

fn main() {
    let dir = guard::dir();
    let server = match guard::claim(&dir) {
        Ok(Claim::First(server)) => server,
        Ok(Claim::Second) => {
            eprintln!("chaude-gui is already running; asked it to focus");
            return;
        }
        Err(e) => {
            eprintln!("chaude-gui: single-instance guard failed: {e}");
            std::process::exit(1);
        }
    };

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let focus_proxy = event_loop.create_proxy();
    guard::serve(server, move || {
        let _ = focus_proxy.send_event(UserEvent::Focus);
    });

    // Engine and job threads call wake from any thread; the proxy sits behind a Mutex to be Sync.
    let wake_proxy = Mutex::new(event_loop.create_proxy());
    let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
        if let Ok(proxy) = wake_proxy.lock() {
            let _ = proxy.send_event(UserEvent::Wake);
        }
    });
    let mut front = Front::start(wake);

    let window = WindowBuilder::new()
        .with_title("chaude")
        .with_inner_size(LogicalSize::new(1000.0, 720.0))
        .build(&event_loop)
        .expect("create window");

    // The IPC handler runs on the window thread; it hands the message to the loop, which routes it to Front.
    let proxy = event_loop.create_proxy();
    let webview = WebViewBuilder::new()
        .with_html(page::html())
        .with_ipc_handler(move |req| {
            let _ = proxy.send_event(UserEvent::FromPage(req.into_body()));
        })
        .build(&window)
        .expect("create webview");

    let quit_at = std::env::var("CHAUDE_GUI_QUIT_AFTER_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(|s| Instant::now() + Duration::from_secs(s));

    // Smoke only: CHAUDE_GUI_SMOKE_START=chat or code:<folder> starts a session at 1 second, then the submit at 2.
    let smoke_start =
        std::env::var("CHAUDE_GUI_SMOKE_START")
            .ok()
            .map(|s| match s.strip_prefix("code:") {
                Some(folder) => {
                    serde_json::json!({ "type": "start", "mode": "code", "folder": folder })
                        .to_string()
                }
                None => serde_json::json!({ "type": "start", "mode": "chat" }).to_string(),
            });
    let smoke_submit = std::env::var("CHAUDE_GUI_SMOKE_SUBMIT").ok();
    if smoke_start.is_some() || smoke_submit.is_some() {
        let proxy = event_loop.create_proxy();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(1));
            if let Some(body) = smoke_start {
                let _ = proxy.send_event(UserEvent::FromPage(body));
            }
            std::thread::sleep(Duration::from_secs(1));
            if let Some(text) = smoke_submit {
                let body = serde_json::json!({ "type": "submit", "text": text }).to_string();
                let _ = proxy.send_event(UserEvent::FromPage(body));
            }
        });
    }

    let probe_at = quit_at.and_then(|at| at.checked_sub(Duration::from_secs(2)));
    let mut probed = false;
    // Set once the run is closing; the quit line prints once, not on every later loop callback.
    let mut quitting = false;
    event_loop.run(move |event, _, control_flow| {
        let next = [quit_at, probe_at.filter(|_| !probed)]
            .into_iter()
            .flatten()
            .min();
        *control_flow = match next {
            Some(at) => ControlFlow::WaitUntil(at),
            None => ControlFlow::Wait,
        };
        if !probed && probe_at.is_some_and(|at| Instant::now() >= at) {
            probed = true;
            let _ = webview.evaluate_script_with_callback(PROBE_JS, |counts| {
                eprintln!("dom: {counts}");
            });
        }
        match event {
            Event::UserEvent(UserEvent::FromPage(body)) => match bridge::parse(&body) {
                Ok(cmd) => {
                    let now = Instant::now();
                    let mut events = front.command(cmd, now);
                    events.extend(front.pump(now));
                    deliver(&webview, &events);
                }
                Err(e) => eprintln!("ipc: bad page message: {e}"),
            },
            Event::UserEvent(UserEvent::Wake) => {
                let events = front.pump(Instant::now());
                deliver(&webview, &events);
            }
            Event::UserEvent(UserEvent::Focus) => {
                eprintln!("guard: second launch asked to focus");
                window.set_focus();
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::WindowEvent {
                event: WindowEvent::Focused(on),
                ..
            } => front.set_focused(on),
            _ => {}
        }
        if !quitting && front.quit {
            quitting = true;
            eprintln!("quit: the run asked to close");
        }
        if !quitting && quit_at.is_some_and(|at| Instant::now() >= at) {
            quitting = true;
            eprintln!("quit: smoke timer reached");
        }
        if quitting {
            *control_flow = ControlFlow::Exit;
        }
    });
}

// Sends the events to the page, one script call each, and logs their kinds on stderr.
fn deliver(webview: &wry::WebView, events: &[chaude_core::gui_bridge::Event]) {
    for ev in events {
        eprintln!("event: {}", ev.kind());
        let _ = webview.evaluate_script(&bridge::script(ev));
    }
}
