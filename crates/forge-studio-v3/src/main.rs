use std::sync::mpsc;
use std::time::Duration;
use std::path::Path;
use std::io::{Read, Write};
use std::net::{TcpStream, TcpListener};
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::window::WindowBuilder;
use wry::WebViewBuilder;

use forge_studio_v3::wire_client;

const STUDIO_HTML: &str = include_str!(concat!(env!("OUT_DIR"), "/studio_face.html"));
const STUDIO_INSTANCE_PORT: u16 = 59013;

fn js_quote(s: &str) -> String {
    let mut q = String::with_capacity(s.len() + 2);
    q.push('"');
    for ch in s.chars() {
        match ch {
            '"' => q.push_str("\\\""),
            '\\' => q.push_str("\\\\"),
            '\n' => q.push_str("\\n"),
            '\r' => q.push_str("\\r"),
            c if (c as u32) < 0x20 => {
                q.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => q.push(c),
        }
    }
    q.push('"');
    q
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Single-instance guard: bind TCP listener exclusively; if already bound, exit gracefully.
    let _instance_guard = match TcpListener::bind(format!("127.0.0.1:{}", STUDIO_INSTANCE_PORT)) {
        Ok(listener) => {
            // We hold this listener for the duration of the program; it prevents other instances.
            listener
        }
        Err(_) => {
            eprintln!("STUDIO already running (port {} bound)", STUDIO_INSTANCE_PORT);
            // Could attempt to focus existing window here; for now just exit.
            std::process::exit(1);
        }
    };

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("13FORGE STUDIO — 3 Gemmas")
        .with_inner_size(tao::dpi::LogicalSize::new(1400.0, 900.0))
        .build(&event_loop)?;

    let (tx_ipc, rx_ipc) = mpsc::channel::<String>();
    let (tx_reply, rx_reply) = mpsc::channel::<String>();
    let (tx_badge, rx_badge) = mpsc::channel::<String>();

    let tx_badge_bg = tx_badge.clone();
    let _status_thread = std::thread::spawn(move || {
        let poll_interval = Duration::from_secs(2);
        let mut last_status = String::new();
        loop {
            std::thread::sleep(poll_interval);
            let status = poll_sidecar_status();
            if status != last_status {
                let _ = tx_badge_bg.send(format!("badge-status:{}", status));
                last_status = status;
            }
        }
    });

    let webview = WebViewBuilder::new()
        .with_html(STUDIO_HTML)
        .with_ipc_handler(move |req| {
            let _ = tx_ipc.send(req.body().clone());
        })
        .build(&window)?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }

        while let Ok(msg) = rx_ipc.try_recv() {
            if let Some(prompt) = msg.strip_prefix("infer:") {
                let prompt = prompt.to_string();
                let tx_reply_clone = tx_reply.clone();
                std::thread::spawn(move || {
                    let result = match wire_client::infer(&prompt, 3000, 35000) {
                        Ok(reply) => reply,
                        Err(e) => format!("FAULT: {}", e),
                    };
                    let _ = tx_reply_clone.send(format!("append:{}", result));
                });
            } else if let Some(pty_msg) = msg.strip_prefix("pty:") {
                // Simple echo for now; full ConPTY integration is phase C.
                if let Some(tab_rest) = pty_msg.strip_prefix("input:") {
                    if let Some((tab_str, data)) = tab_rest.split_once(':') {
                        if let Ok(_tab) = tab_str.parse::<usize>() {
                            // Echo the input back to the tab with command prompt format
                            let echo = format!("$ {}\n", data);
                            let _ = tx_reply.send(format!("hearth:{}", echo));
                        }
                    }
                }
            } else if msg == "probe:daemon" {
                let is_up = check_daemon_open();
                let badge_cmd = if is_up { "badge:up" } else { "badge:down" };
                let _ = tx_reply.send(badge_cmd.to_string());
            } else if msg == "toggle:theme" {
                let _ = tx_reply.send("theme:toggle".to_string());
            }
        }

        while let Ok(reply) = rx_reply.try_recv() {
            if let Some(text) = reply.strip_prefix("append:") {
                let html = format!(
                    "window.triadAppend&&window.triadAppend({})",
                    js_quote(text)
                );
                let _ = webview.evaluate_script(&html);
            } else if let Some(hearth_text) = reply.strip_prefix("hearth:") {
                let js = format!(
                    "window.appendHearth&&window.appendHearth(0, {})",
                    js_quote(hearth_text)
                );
                let _ = webview.evaluate_script(&js);
            } else if let Some(state) = reply.strip_prefix("badge:") {
                let js = format!(
                    "window.updateBadgeStatus&&window.updateBadgeStatus({})",
                    js_quote(state)
                );
                let _ = webview.evaluate_script(&js);
            } else if reply == "theme:toggle" {
                let _ = webview.evaluate_script(
                    "document.body.classList.toggle('permafrost'); \
                     localStorage.setItem('theme', \
                     document.body.classList.contains('permafrost') ? 'permafrost' : 'molten')"
                );
            }
        }

        while let Ok(badge_msg) = rx_badge.try_recv() {
            if let Some(status) = badge_msg.strip_prefix("badge-status:") {
                let js = format!(
                    "window.updateBadgeStatus&&window.updateBadgeStatus({})",
                    js_quote(status)
                );
                let _ = webview.evaluate_script(&js);
            }
        }
    });
}

fn check_daemon_open() -> bool {
    let addr = "127.0.0.1:13013";
    match addr.parse::<std::net::SocketAddr>() {
        Ok(socket_addr) => {
            match std::net::TcpStream::connect_timeout(&socket_addr, Duration::from_millis(500)) {
                Ok(_) => true,
                Err(_) => false,
            }
        }
        Err(_) => false,
    }
}

fn poll_sidecar_status() -> String {
    let root = match std::env::current_dir() {
        Ok(d) => d,
        Err(_) => return "OFFLINE".to_string(),
    };

    match read_endpoint(&root) {
        Some(addr) => {
            match status_frame(&addr) {
                Some(reply) => {
                    match parse_resident(&reply) {
                        Some(true) => "RESIDENT".to_string(),
                        Some(false) => "UP".to_string(),
                        None => "OFFLINE".to_string(),
                    }
                }
                None => "OFFLINE".to_string(),
            }
        }
        None => "OFFLINE".to_string(),
    }
}

fn read_endpoint(root: &Path) -> Option<String> {
    let path = root.join(".forge/v3-directives.ron");
    let body = std::fs::read_to_string(path).ok()?;
    let line = body.lines().find(|l| l.trim_start().starts_with("gemma_endpoint:"))?;
    let (_, rest) = line.split_once(':')?;
    let start = rest.find('"')? + 1;
    let end = start + rest[start..].find('"')?;
    rest[start..end].strip_prefix("http://").map(str::to_string)
}

fn status_frame(addr: &str) -> Option<String> {
    const CONNECT_TIMEOUT: Duration = Duration::from_millis(300);
    const READ_TIMEOUT: Duration = Duration::from_millis(300);

    let mut stream = TcpStream::connect_timeout(&addr.parse().ok()?, CONNECT_TIMEOUT).ok()?;
    stream.set_read_timeout(Some(READ_TIMEOUT)).ok()?;
    stream.set_write_timeout(Some(READ_TIMEOUT)).ok()?;

    let payload = b"STATUS";
    stream.write_all(&(payload.len() as u32).to_be_bytes()).ok()?;
    stream.write_all(payload).ok()?;

    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes).ok()?;
    let len = u32::from_be_bytes(len_bytes) as usize;
    if len > 4096 {
        return None;
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).ok()?;
    String::from_utf8(buf).ok()
}

fn parse_resident(reply: &str) -> Option<bool> {
    reply
        .split_whitespace()
        .find_map(|tok| tok.strip_prefix("resident="))
        .map(|v| v == "yes")
}
