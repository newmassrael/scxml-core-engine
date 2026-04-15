// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default)]
pub struct CodecLittleEndian {
    pub sensor_id: u8,
    pub value: u16,
    pub status: u8,
}

#[allow(dead_code)]
impl CodecLittleEndian {
    /// Construct an instance with every field zero-initialized via
    /// [`Default`]. Generated procedure_l2 code stores codec instances
    /// as owned members and needs an infallible constructor to
    /// initialize them before any `encode()` or `decode()` call.
    pub fn new() -> Self {
        Self::default()
    }

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
