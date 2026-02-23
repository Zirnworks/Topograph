/// Authoritative heightmap. Row-major: index = y * width + x.
/// Heights are in [0.0, 1.0] range.
pub struct Heightmap {
    pub data: Vec<f32>,
    pub width: u32,
    pub height: u32,
}

impl Heightmap {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            data: vec![0.0; (width * height) as usize],
            width,
            height,
        }
    }

    pub fn get(&self, x: u32, y: u32) -> f32 {
        self.data[(y * self.width + x) as usize]
    }

    pub fn set(&mut self, x: u32, y: u32, val: f32) {
        self.data[(y * self.width + x) as usize] = val;
    }
}
