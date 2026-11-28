//! Starts the Website Manager desktop app.

// No console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    website_manager_desktop::run();
}
