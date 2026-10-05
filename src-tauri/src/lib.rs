pub mod database;
pub mod money;
pub mod quantity;

use database::Database;
use serde::Serialize;
use tauri::Manager;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StartupHealth {
    ready: bool,
    database: &'static str,
    app_version: &'static str,
}

#[tauri::command]
async fn startup_health_check(
    database: tauri::State<'_, Database>,
) -> Result<StartupHealth, String> {
    let ready = database.is_healthy().await;
    Ok(StartupHealth {
        ready,
        database: if ready { "ready" } else { "error" },
        app_version: env!("CARGO_PKG_VERSION"),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }));
    }

    builder
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;
            let database_path = app_data_dir.join("tillbahrain.db");
            let database = tauri::async_runtime::block_on(Database::open(database_path))?;
            app.manage(database);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![startup_health_check])
        .run(tauri::generate_context!())
        .expect("Tillbahrain application runtime failed");
}
