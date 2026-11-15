//! chaude GUI entry point. One native window (tao) with the embedded page (wry webview).
//! CHAUDE_GUI_QUIT_AFTER_SECS=<n> closes the window after n seconds (smoke runs only).

use std::time::{Duration, Instant};

use chaude_gui::bridge::{self, FromPage, ToPage};
use chaude_gui::guard::{self, Claim};
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use wry::WebViewBuilder;

const PAGE: &str = include_str!("../web/index.html");

// Events the window thread gets from other threads: the page's messages and a second launch's focus request.
#[derive(Debug)]
enum UserEvent {
    FromPage(String),
    Focus,
}

fn main() {
    let listener = match guard::claim() {
        Ok(Claim::First(listener)) => listener,
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
    let proxy = event_loop.create_proxy();
    guard::serve(listener, move || {
        let _ = proxy.send_event(UserEvent::Focus);
    });

    let window = WindowBuilder::new()
        .with_title("chaude")
        .with_inner_size(LogicalSize::new(1000.0, 720.0))
        .build(&event_loop)
        .expect("create window");

    // The IPC handler runs on the window thread; it hands the message to the loop, which answers the page.
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

    event_loop.run(move |event, _, control_flow| {
        *control_flow = match quit_at {
            Some(at) => ControlFlow::WaitUntil(at),
            None => ControlFlow::Wait,
        };
        match event {
            Event::UserEvent(UserEvent::FromPage(body)) => match bridge::parse(&body) {
                Ok(FromPage::Ping) => {
                    eprintln!("ipc: ping from page");
                    let _ = webview.evaluate_script(&bridge::script(&ToPage::Pong));
                }
                Err(e) => eprintln!("ipc: bad page message: {e}"),
            },
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
        if quit_at.is_some_and(|at| Instant::now() >= at) {
            eprintln!("quit: smoke timer reached");
            *control_flow = ControlFlow::Exit;
        }
    });
}
