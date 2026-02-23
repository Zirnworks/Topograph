use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::ipc::Response;
use tauri::State;
use crate::erosion::{hydraulic, thermal};
use crate::erosion::hydraulic::HydraulicParams;
use crate::erosion::thermal::ThermalParams;
use crate::ipc;
use crate::noise_gen::{self, NoiseParams};
use crate::sculpt::{self, BrushStroke};
use crate::state::AppState;

#[tauri::command]
pub fn get_heightmap(state: State<'_, AppState>) -> Response {
    let hm = state.heightmap.lock().unwrap();
    Response::new(ipc::pack_full(&hm))
}

#[tauri::command]
pub fn apply_brush_stroke(stroke: BrushStroke, state: State<'_, AppState>) -> Response {
    let mut hm = state.heightmap.lock().unwrap();
    let (rx, ry, rw, rh) = sculpt::apply_brush(&mut hm, &stroke);
    if rw == 0 || rh == 0 {
        return Response::new(ipc::pack_full(&hm));
    }
    Response::new(ipc::pack_region(&hm, rx, ry, rw, rh))
}

#[tauri::command]
pub fn generate_terrain(params: NoiseParams, state: State<'_, AppState>) -> Response {
    let mut hm = state.heightmap.lock().unwrap();
    noise_gen::generate_terrain(&mut hm, &params);
    Response::new(ipc::pack_full(&hm))
}

#[tauri::command]
pub fn run_thermal_erosion(params: ThermalParams, state: State<'_, AppState>) -> Response {
    let mut hm = state.heightmap.lock().unwrap();
    thermal::erode(&mut hm, &params);
    Response::new(ipc::pack_full(&hm))
}

#[tauri::command]
pub fn run_hydraulic_erosion(
    params: HydraulicParams,
    state: State<'_, AppState>,
    channel: tauri::ipc::Channel<f32>,
) -> Result<(), String> {
    if state
        .erosion_running
        .swap(true, Ordering::SeqCst)
    {
        return Err("Erosion already running".to_string());
    }
    state.erosion_abort.store(false, Ordering::SeqCst);

    let hm = Arc::clone(&state.heightmap);
    let abort = Arc::clone(&state.erosion_abort);
    let running = Arc::clone(&state.erosion_running);

    std::thread::spawn(move || {
        {
            let mut hm_guard = hm.lock().unwrap();
            hydraulic::erode(&mut hm_guard, &params, &abort, &|progress| {
                let _ = channel.send(progress);
            });
        }
        running.store(false, Ordering::SeqCst);
    });

    Ok(())
}

#[tauri::command]
pub fn abort_erosion(state: State<'_, AppState>) {
    state.erosion_abort.store(true, Ordering::SeqCst);
}
