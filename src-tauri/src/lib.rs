//! Tauri обвивка: опакова SPA-та, носи native команди през tauri-specta.
//! Фаза 0: една probe команда + export на TS bindings.

#[cfg(debug_assertions)]
use specta_typescript::Typescript;
use tauri_specta::{collect_commands, Builder};

#[tauri::command]
#[specta::specta]
fn probe() -> String {
    format!("dzambo desktop · {}", std::env::consts::OS)
}

pub fn run() {
    let builder = Builder::<tauri::Wry>::new().commands(collect_commands![probe]);

    #[cfg(debug_assertions)]
    builder
        .export(
            Typescript::default(),
            "../frontend/src/bindings/commands.ts",
        )
        .expect("експорт на TS bindings");

    tauri::Builder::default()
        .invoke_handler(builder.invoke_handler())
        .run(tauri::generate_context!())
        .expect("dzambo desktop");
}
