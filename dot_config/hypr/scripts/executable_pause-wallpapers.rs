#!/usr/bin/env -S /home/leoc/.cargo/bin/cargo +nightly -Zscript --quiet
---
[package]
edition = "2024"

[dependencies]
serde_json = "=1.0.151"
---

use serde_json::{Value, json};
use std::{env, error::Error, io::{BufRead, BufReader, Write}, os::unix::net::UnixStream,
    process::Command, thread, time::Duration};

const SCENES: [(&str, &str); 3] = [
    ("#ASN4JVmqxLLd", "aurora-lake"),
    ("#GjMYMxgwABkB", "tranquil-japan-lake-view"),
    ("#GrMYMxgwABcY", "blue-mountain-horizons"),
];

fn hyprland_state(command: &str) -> Result<Value, Box<dyn Error>> {
    let output = Command::new("hyprctl").args([command, "-j"]).output()?;
    if !output.status.success() {
        return Err(format!("hyprctl {command} failed").into());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn update(once: bool) -> Result<(), Box<dyn Error>> {
    let monitors = hyprland_state("monitors")?;
    let clients = hyprland_state("clients")?;
    let clients = clients.as_array().ok_or("invalid clients response")?;
    let runtime = env::var("XDG_RUNTIME_DIR")?;
    for monitor in monitors.as_array().ok_or("invalid monitors response")? {
        let Some((_, scene)) = SCENES.iter().find(|(serial, _)| monitor["serial"] == *serial) else {
            continue;
        };
        let fullscreen = clients.iter().any(|client| {
            client["mapped"] == true && client["hidden"] != true
                && client["monitor"] == monitor["id"]
                && client["fullscreen"].as_u64().unwrap_or(0) > 0
                && (client["workspace"]["id"] == monitor["activeWorkspace"]["id"]
                    || client["workspace"]["id"] == monitor["specialWorkspace"]["id"])
        });
        let pause = fullscreen || monitor["dpmsStatus"] == false;
        let path = format!("{runtime}/wallpapers-triple/{scene}.sock");
        let Ok(mut stream) = UnixStream::connect(path) else { continue };
        stream.set_read_timeout(Some(Duration::from_millis(500)))?;
        stream.set_write_timeout(Some(Duration::from_millis(500)))?;
        writeln!(stream, "{}", json!({"command": ["set_property", "pause", pause]}))?;
        let mut response = String::new();
        BufReader::new(stream).read_line(&mut response)?;
        let response: Value = serde_json::from_str(&response)?;
        if response["error"] != "success" {
            return Err(format!("{scene}: mpv rejected pause change: {response}").into());
        }
        if once {
            println!("{scene}: pause={pause}");
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let once = env::args().any(|arg| arg == "--once");
    if once { return update(true); }
    loop {
        if let Err(error) = update(false) {
            eprintln!("Wallpaper pause: {error}");
        }
        thread::sleep(Duration::from_secs(2));
    }
}
