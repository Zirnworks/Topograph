mod ai;
mod commands;
mod erosion;
mod heightmap;
mod ipc;
mod noise_gen;
mod sculpt;
mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(state::AppState::new())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_heightmap,
            commands::apply_brush_stroke,
            commands::generate_terrain,
            commands::run_thermal_erosion,
            commands::run_hydraulic_erosion,
            commands::abort_erosion,
            commands::run_depth_estimation,
            commands::run_inpainting,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Topograph");
}
