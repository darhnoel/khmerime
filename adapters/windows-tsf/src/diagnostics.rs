//! Lightweight diagnostics for developer TSF smoke testing.

use std::fs::OpenOptions;
use std::io::Write;

pub const LOG_PATH: &str = "C:\\Temp\\khmerime-tsf.log";

pub fn log(message: impl AsRef<str>) {
    let _ = std::fs::create_dir_all("C:\\Temp");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(LOG_PATH) {
        let _ = writeln!(
            file,
            "[{}] {}",
            timestamp_millis(),
            message.as_ref().replace(['\r', '\n'], " ")
        );
    }
}

fn timestamp_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
