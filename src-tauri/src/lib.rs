mod ai;
mod commands;
mod erosion;
mod heightmap;
mod ipc;
mod noise_gen;
mod project;
mod sculpt;
mod state;

use tauri::menu::{AboutMetadata, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(state::AppState::new())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // macOS app menu
            let app_menu = SubmenuBuilder::new(app, "Topograph")
                .about(Some(AboutMetadata::default()))
                .separator()
                .hide()
                .hide_others()
                .show_all()
                .separator()
                .quit()
                .build()?;

            // File menu
            let save_item = MenuItemBuilder::new("Save Project")
                .id("save")
                .accelerator("CmdOrCtrl+S")
                .build(app)?;
            let open_item = MenuItemBuilder::new("Open Project")
                .id("open")
                .accelerator("CmdOrCtrl+O")
                .build(app)?;

            let file_menu = SubmenuBuilder::new(app, "File")
                .item(&save_item)
                .item(&open_item)
                .separator()
                .text("export_png16", "Export Heightmap (PNG 16-bit)")
                .text("export_raw", "Export Heightmap (Raw f32)")
                .build()?;

            let edit_menu = SubmenuBuilder::new(app, "Edit")
                .undo()
                .redo()
                .separator()
                .cut()
                .copy()
                .paste()
                .select_all()
                .build()?;

            let menu = MenuBuilder::new(app)
                .items(&[&app_menu, &file_menu, &edit_menu])
                .build()?;

            // Set menu on the main window directly
            if let Some(window) = app.get_webview_window("main") {
                window.set_menu(menu)?;
                window.on_menu_event(move |window, event| {
                    let id = event.id().0.as_str();
                    let _ = window.emit("menu-action", id);
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_heightmap,
            commands::apply_brush_stroke,
            commands::generate_terrain,
            commands::run_thermal_erosion,
            commands::run_hydraulic_erosion,
            commands::abort_erosion,
            commands::run_depth_estimation,
            commands::run_inpainting,
            commands::save_project,
            commands::load_project,
            commands::export_heightmap,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Topograph");
}
