use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use crate::heightmap::Heightmap;

pub struct AppState {
    pub heightmap: Arc<Mutex<Heightmap>>,
    pub erosion_abort: Arc<AtomicBool>,
    pub erosion_running: Arc<AtomicBool>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            heightmap: Arc::new(Mutex::new(Heightmap::new(512, 512))),
            erosion_abort: Arc::new(AtomicBool::new(false)),
            erosion_running: Arc::new(AtomicBool::new(false)),
        }
    }
}
