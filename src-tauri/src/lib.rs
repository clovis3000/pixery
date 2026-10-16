#[cfg(feature = "gui")]
use std::sync::Mutex;

pub mod archive;
#[cfg(feature = "gui")]
mod commands;
pub mod db;
pub mod models;
pub mod providers;
#[cfg(feature = "gui")]
mod watcher;
pub mod workflow;

pub mod cli;

#[cfg(feature = "gui")]
use commands::AppState;

#[cfg(feature = "gui")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Ensure directories exist
    archive::ensure_dirs().expect("Failed to create archive directories");

    // Open database
    let db = db::Database::open(&archive::db_path()).expect("Failed to open database");

    // Cleanup stale job records on startup
    if let Ok(stalled) = db.cleanup_stalled_jobs() {
        if stalled > 0 {
            eprintln!("Cleaned up {} stalled jobs", stalled);
        }
    }
    if let Ok(old) = db.cleanup_old_jobs(24) {
        if old > 0 {
            eprintln!("Cleaned up {} old completed/failed jobs", old);
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState { db: Mutex::new(db) })
        .invoke_handler(tauri::generate_handler![
            commands::generate_image,
            commands::list_generations,
            commands::search_generations,
            commands::get_generation,
            commands::toggle_starred,
            commands::trash_generation,
            commands::trash_generations,
            commands::restore_generation,
            commands::permanently_delete_generation,
            commands::update_prompt,
            commands::update_title,
            commands::add_tags,
            commands::remove_tag,
            commands::list_tags,
            commands::list_models,
            commands::get_cost_summary,
            commands::get_image_path,
            commands::get_references,
            commands::list_jobs,
            commands::list_failed_jobs,
            commands::list_collections,
            commands::create_collection,
            commands::add_to_collection,
            commands::remove_from_collection,
            commands::delete_collection,
            commands::prompt_history,
            commands::save_to_folder,
            commands::get_selfhosted_url,
            commands::set_selfhosted_url,
            commands::check_selfhosted_health,
        ])
        .setup(|app| {
            // Start file watcher for auto-refresh
            let generations_dir = archive::generations_dir();
            watcher::start_watcher(app.handle().clone(), &generations_dir);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
