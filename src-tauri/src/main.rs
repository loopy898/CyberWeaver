//! CyberWeaver application entry point.
//!
//! This binary crate uses the `tauri_app_lib` library for initialization.
//! The actual app bootstrap logic lives in `lib.rs`.

// Prevent the console window from appearing on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = tauri_app_lib::run() {
        eprintln!("failed to run tauri application: {error}");
        std::process::exit(1);
    }
}
