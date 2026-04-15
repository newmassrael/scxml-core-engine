// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default)]
pub struct CodecSimpleFrame {
    pub msg_id: u8,
    pub length: u8,
    pub payload: u16,
}

#[allow(dead_code)]
impl CodecSimpleFrame {
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
            msg_id: raw[0],
            length: raw[1],
            payload: ((raw[2] as u16) << 8) | raw[3] as u16,
        })
    }

    pub fn encode(&self) -> Vec<u8> {
        vec![
            self.msg_id,
            self.length,
            (self.payload >> 8 & 0xFF) as u8,
            (self.payload & 0xFF) as u8,
        ]
    }
}
