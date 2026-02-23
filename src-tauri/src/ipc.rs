use crate::heightmap::Heightmap;

pub const IPC_VERSION: u32 = 1;
pub const MSG_FULL: u8 = 0;
pub const MSG_REGION: u8 = 1;

/// Pack the full heightmap into binary IPC format.
/// Format: [version:u32 LE][type:u8][pad:3B][width:u32 LE][height:u32 LE][data: w*h f32 LE]
pub fn pack_full(hm: &Heightmap) -> Vec<u8> {
    let data_bytes = hm.data.len() * 4;
    let header_size = 16; // 4 + 1 + 3 + 4 + 4
    let mut buf = Vec::with_capacity(header_size + data_bytes);

    buf.extend_from_slice(&IPC_VERSION.to_le_bytes());
    buf.push(MSG_FULL);
    buf.extend_from_slice(&[0u8; 3]); // padding
    buf.extend_from_slice(&hm.width.to_le_bytes());
    buf.extend_from_slice(&hm.height.to_le_bytes());

    for &val in &hm.data {
        buf.extend_from_slice(&val.to_le_bytes());
    }

    buf
}

/// Pack a rectangular sub-region for partial updates.
/// Format: [version:u32 LE][type:u8][pad:3B][x:u32][y:u32][w:u32][h:u32][data: w*h f32 LE]
pub fn pack_region(hm: &Heightmap, rx: u32, ry: u32, rw: u32, rh: u32) -> Vec<u8> {
    let data_bytes = (rw * rh) as usize * 4;
    let header_size = 24; // 4 + 1 + 3 + 4 + 4 + 4 + 4
    let mut buf = Vec::with_capacity(header_size + data_bytes);

    buf.extend_from_slice(&IPC_VERSION.to_le_bytes());
    buf.push(MSG_REGION);
    buf.extend_from_slice(&[0u8; 3]); // padding
    buf.extend_from_slice(&rx.to_le_bytes());
    buf.extend_from_slice(&ry.to_le_bytes());
    buf.extend_from_slice(&rw.to_le_bytes());
    buf.extend_from_slice(&rh.to_le_bytes());

    for y in ry..(ry + rh) {
        for x in rx..(rx + rw) {
            let val = hm.get(x, y);
            buf.extend_from_slice(&val.to_le_bytes());
        }
    }

    buf
}
