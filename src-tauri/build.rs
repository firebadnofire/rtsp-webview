use std::{env, fs, path::PathBuf};

fn ensure_ui_dist_dir() {
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be available during build"),
    );
    let dist_dir = manifest_dir.join("../ui/dist");
    fs::create_dir_all(&dist_dir).expect("failed to create ui/dist directory for Tauri build");
}

fn main() {
    ensure_ui_dist_dir();
    tauri_build::build()
}
