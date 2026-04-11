// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Do not edit — regenerate from the source SCXML file.

#[allow(dead_code)]
pub struct CodecLittleEndian {
    pub sensor_id: u8,
    pub value: u16,
    pub status: u8,
}

#[allow(dead_code)]
impl CodecLittleEndian {
    pub fn decode(raw: &[u8]) -> Option<Self> {
        if raw.len() < 4 {
            return None;
        }
        Some(Self {
            sensor_id: raw[0],
            value: raw[1] as u16 | ((raw[2] as u16) << 8),
            status: raw[3],
        })
    }

    pub fn encode(&self) -> Vec<u8> {
        vec![
            self.sensor_id,
            (self.value & 0xFF) as u8,
            (self.value >> 8 & 0xFF) as u8,
            self.status,
        ]
    }
}
