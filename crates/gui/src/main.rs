//! chaude GUI entry point. One native window (tao) with the embedded page (wry webview).
//! CHAUDE_GUI_QUIT_AFTER_SECS=<n> closes the window after n seconds (smoke runs only).
//! CHAUDE_GUI_SMOKE_SUBMIT=<text> sends that text as a submit command 2 seconds after start (smoke runs only).

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chaude_gui::bridge;
use chaude_gui::front::Front;
use chaude_gui::guard::{self, Claim};
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use wry::WebViewBuilder;

const PAGE: &str = include_str!("../web/index.html");

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
        .with_html(PAGE)
        .with_ipc_handler(move |req| {
            let _ = proxy.send_event(UserEvent::FromPage(req.into_body()));
        })
        .build(&window)
        .expect("create webview");

    let quit_at = std::env::var("CHAUDE_GUI_QUIT_AFTER_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(|s| Instant::now() + Duration::from_secs(s));

    if let Ok(text) = std::env::var("CHAUDE_GUI_SMOKE_SUBMIT") {
        let proxy = event_loop.create_proxy();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(2));
            let body = serde_json::json!({ "type": "submit", "text": text }).to_string();
            let _ = proxy.send_event(UserEvent::FromPage(body));
        });
    }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = match quit_at {
            Some(at) => ControlFlow::WaitUntil(at),
            None => ControlFlow::Wait,
        };
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
            _ => {}
        }
        if front.quit {
            eprintln!("quit: the run asked to close");
            *control_flow = ControlFlow::Exit;
        }
        if quit_at.is_some_and(|at| Instant::now() >= at) {
            eprintln!("quit: smoke timer reached");
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
