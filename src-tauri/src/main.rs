#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_state;
mod commands;
mod errors;
mod events;
mod startup_checks;
mod state;
mod stub_streams;

use app_state::ManagedState;

fn main() {
    let context = tauri::generate_context!();
    if let Err(message) = startup_checks::preflight_frontend(&context) {
        eprintln!("Frontend preflight failed: {}", message);
        std::process::exit(1);
    }

    tauri::Builder::default()
        .manage(ManagedState::new())
        .invoke_handler(tauri::generate_handler![
            commands::set_active_screen,
            commands::set_active_panel,
            commands::get_state,
            commands::update_panel_config,
            commands::update_stream_defaults,
            commands::set_panel_secret,
            commands::auto_populate_cameras,
            commands::start_stream,
            commands::stop_stream,
            commands::start_screen,
            commands::stop_screen,
            commands::start_all_global,
            commands::stop_all_global,
            commands::save_config,
            commands::load_config,
            commands::load_startup_config,
            commands::snapshot,
            commands::toggle_recording,
            commands::toggle_fullscreen,
            commands::create_screen,
            commands::delete_screen,
        ])
        .run(context)
        .expect("error while running tauri application");
}
