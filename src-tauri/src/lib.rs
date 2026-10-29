mod commands;

use pharos_engine::Engine;
use commands::AppState;
use tauri::Manager;
use tauri_specta::{Builder, collect_commands};

pub const BINDINGS_PATH: &str = "../app/src/bindings.ts";

#[tauri::command]
#[specta::specta]
fn ping() -> String {
    "pong".into()
}

/// Single source of truth for the IPC surface. Bindings are generated from this.
pub fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .disable_serde_phases()
        .commands(collect_commands![
            ping,
            commands::git_version,
            commands::get_source,
            commands::add_source,
            commands::plan_sync,
            commands::pending_sync,
            commands::apply_sync,
            commands::discard_sync,
            commands::list_widgets,
            commands::list_dashboards,
            commands::get_platform,
            commands::run_widget,
        ])
}

pub fn export_bindings(builder: &Builder<tauri::Wry>, path: &str) {
    builder
        .export(specta_typescript::Typescript::default(), path)
        .expect("failed to export typescript bindings");
}

pub fn run() {
    let builder = specta_builder();

    #[cfg(debug_assertions)]
    export_bindings(&builder, BINDINGS_PATH);

    tauri::Builder::default()
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            let engine = Engine::open(&app.path().app_data_dir()?)?;
            app.manage(AppState::new(engine));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_returns_pong() {
        assert_eq!(ping(), "pong");
    }

    /// Regenerates bindings; CI fails if the committed file drifts (`git diff --exit-code`).
    #[test]
    fn export_bindings_up_to_date() {
        export_bindings(&specta_builder(), BINDINGS_PATH);
    }
}
