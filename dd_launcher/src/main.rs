// Double Dragon launcher: shows the controls page in a window, saves the
// settings next to the app, and starts touchHLE without a console window.
// Folder layout (next to DoubleDragon.exe):
//   engine/   touchHLE.exe and its files
//   game/     put DoubleDragon.ipa here
//   config/   saved controls (controls.json)
//   saves/    player1 and player2 save data
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use wry::http::{header::CONTENT_TYPE, Request, Response};
use wry::{WebContext, WebViewBuilder};

const PAGE: &str = include_str!("../ui/index.html");

enum UserEvent {
    Ipc(String),
    Status(String),
}

fn base_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn find_ipa(game: &Path) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(game)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("ipa"))
        })
        .collect();
    found.sort();
    found.into_iter().next()
}

fn engine_exe(engine: &Path) -> PathBuf {
    if cfg!(windows) {
        engine.join("touchHLE.exe")
    } else {
        engine.join("touchHLE")
    }
}

fn js_string(s: &str) -> String {
    serde_json::Value::String(s.to_string()).to_string()
}

fn spawn_engine(engine: &Path, args: &[String]) -> std::io::Result<()> {
    let mut cmd = Command::new(engine_exe(engine));
    cmd.args(args).current_dir(engine);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.spawn().map(|_| ())
}

/// Start one or two game windows. Returns a message for the page.
fn play(base: &Path, msg: &serde_json::Value) -> String {
    let engine = base.join("engine");
    if !engine_exe(&engine).exists() {
        return "The engine folder is missing touchHLE.exe. Re-download the app.".into();
    }
    let Some(ipa) = find_ipa(&base.join("game")) else {
        return "No .ipa found. Put DoubleDragon.ipa in the game folder.".into();
    };
    let count = msg["count"].as_u64().unwrap_or(1);
    let list = |i: usize| -> Vec<String> {
        msg["args"][i]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default()
    };
    let saves = base.join("saves");
    let sandbox = |n: u32| {
        let d = saves.join(format!("player{n}"));
        let _ = std::fs::create_dir_all(&d);
        format!("--sandbox={}", d.display())
    };
    let ipa = ipa.display().to_string();
    if count >= 2 {
        let mut a = vec![ipa.clone(), "--gamekit-host=47800".into(), sandbox(1)];
        a.extend(list(0));
        if let Err(e) = spawn_engine(&engine, &a) {
            return format!("Couldn't start the game: {e}");
        }
        std::thread::sleep(Duration::from_secs(2));
        let mut b = vec![ipa, "--gamekit-join=127.0.0.1:47800".into(), sandbox(2)];
        b.extend(list(1));
        if let Err(e) = spawn_engine(&engine, &b) {
            return format!("Player 2's window couldn't start: {e}");
        }
        "Started 2 players.".into()
    } else {
        let mut a = vec![ipa, sandbox(1)];
        a.extend(list(0));
        if let Err(e) = spawn_engine(&engine, &a) {
            return format!("Couldn't start the game: {e}");
        }
        "Started 1 player.".into()
    }
}

fn open_folder(path: &Path) {
    let _ = std::fs::create_dir_all(path);
    #[cfg(windows)]
    let _ = Command::new("explorer").arg(path).spawn();
    #[cfg(target_os = "linux")]
    let _ = Command::new("xdg-open").arg(path).spawn();
}

fn main() -> wry::Result<()> {
    let base = base_dir();
    for d in ["engine", "game", "config", "saves"] {
        let _ = std::fs::create_dir_all(base.join(d));
    }
    let config_file = base.join("config").join("controls.json");
    let config = std::fs::read_to_string(&config_file).unwrap_or_else(|_| "null".into());
    let config = if serde_json::from_str::<serde_json::Value>(&config).is_ok() {
        config
    } else {
        "null".into()
    };
    let ipa = find_ipa(&base.join("game"))
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()));
    let init = format!(
        "window.DD_APP = {{ config: {config}, ipa: {}, folder: {} }};",
        ipa.as_deref().map(js_string).unwrap_or_else(|| "null".into()),
        js_string(&base.display().to_string()),
    );

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let window = WindowBuilder::new()
        .with_title("Double Dragon")
        .with_inner_size(tao::dpi::LogicalSize::new(1180.0, 900.0))
        .build(&event_loop)
        .expect("window");

    let mut ctx = WebContext::new(Some(base.join("config").join("webview")));
    let url = if cfg!(windows) {
        "http://dd.localhost/index.html"
    } else {
        "dd://localhost/index.html"
    };
    let builder = WebViewBuilder::with_web_context(&mut ctx)
        .with_custom_protocol("dd".into(), |_id, _req: Request<Vec<u8>>| {
            Response::builder()
                .header(CONTENT_TYPE, "text/html; charset=utf-8")
                .body(Cow::Borrowed(PAGE.as_bytes()))
                .unwrap()
        })
        .with_initialization_script(&init)
        .with_ipc_handler(move |req: Request<String>| {
            let _ = proxy.send_event(UserEvent::Ipc(req.body().clone()));
        })
        .with_url(url);

    #[cfg(not(target_os = "linux"))]
    let webview = builder.build(&window)?;
    #[cfg(target_os = "linux")]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        builder.build_gtk(window.default_vbox().unwrap())?
    };

    let status_proxy = event_loop.create_proxy();
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::UserEvent(UserEvent::Status(s)) => {
                let _ = webview.evaluate_script(&format!(
                    "window.ddAppStatus && window.ddAppStatus({})",
                    js_string(&s)
                ));
            }
            Event::UserEvent(UserEvent::Ipc(body)) => {
                let Ok(msg) = serde_json::from_str::<serde_json::Value>(&body) else {
                    return;
                };
                match msg["type"].as_str() {
                    Some("save") => {
                        let _ = std::fs::write(&config_file, msg["state"].to_string());
                    }
                    Some("play") => {
                        // Starting player 2 waits 2 s; keep the window responsive.
                        let base = base.clone();
                        let p = status_proxy.clone();
                        std::thread::spawn(move || {
                            let s = play(&base, &msg);
                            let _ = p.send_event(UserEvent::Status(s));
                        });
                    }
                    Some("open") => {
                        let which = msg["folder"].as_str().unwrap_or("game");
                        if ["game", "saves", "config", "engine"].contains(&which) {
                            open_folder(&base.join(which));
                        }
                    }
                    Some("refresh") => {
                        let ipa = find_ipa(&base.join("game")).and_then(|p| {
                            p.file_name().map(|n| n.to_string_lossy().into_owned())
                        });
                        let _ = webview.evaluate_script(&format!(
                            "window.ddAppIpa && window.ddAppIpa({})",
                            ipa.as_deref().map(js_string).unwrap_or_else(|| "null".into())
                        ));
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    });
}
