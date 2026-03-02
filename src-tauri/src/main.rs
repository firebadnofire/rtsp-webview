#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_state;
mod commands;
mod errors;
mod events;
mod state;
mod stub_streams;

use app_state::ManagedState;

fn main() {
    tauri::Builder::default()
        .manage(ManagedState::new())
        .invoke_handler(tauri::generate_handler![
            commands::set_active_screen,
            commands::set_active_panel,
            commands::get_state,
            commands::update_panel_config,
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
            commands::snapshot,
            commands::toggle_recording,
            commands::toggle_fullscreen,
            commands::create_screen,
            commands::delete_screen,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
