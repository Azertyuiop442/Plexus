use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = std::env::var("CC_SIDEBAR_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| cc_dashboard::ipc::data_dir());

    fs::create_dir_all(&data_dir)?;

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;

    let home_cc_mux = std::env::var("HOME")
        .map(|h| PathBuf::from(h).join(".commandcode/bin/cc-mux"))
        .ok();

    let mut cmd = if let Some(path) = home_cc_mux.clone().filter(|p| p.exists()) {
        Command::new(path)
    } else {
        Command::new("cc-mux")
    };

    let mut child: Child = cmd
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap_or_else(|_| {
            Command::new("commandcode")
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .expect("Failed to launch cc-mux or commandcode")
        });

    let child_id = child.id();

    loop {

        if !running.load(Ordering::SeqCst) {

            unsafe {
                libc::kill(child_id as i32, libc::SIGTERM);
            }
            let _ = child.wait();
            std::process::exit(0);
        }

        match child.try_wait() {
            Ok(Some(status)) => {

                std::process::exit(status.code().unwrap_or(0));
            }
            Ok(None) => {

                std::thread::sleep(Duration::from_millis(100));
            }
            Err(_) => {
                std::process::exit(1);
            }
        }
    }
}

