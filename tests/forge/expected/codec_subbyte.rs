// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default)]
pub struct CodecSubbyte {
    pub priority: u8,
    pub channel: u8,
    pub direction: u8,
}

#[allow(dead_code)]
impl CodecSubbyte {
    /// Construct an instance with every field zero-initialized via
    /// [`Default`]. Generated procedure_l2 code stores codec instances
    /// as owned members and needs an infallible constructor to
    /// initialize them before any `encode()` or `decode()` call.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn decode(raw: &[u8]) -> Option<Self> {
        if raw.len() < 1 {
            return None;
        }
        Some(Self {
            priority: (raw[0] >> 5) & 0x07,
            channel: (raw[0] >> 2) & 0x07,
            direction: (raw[0] >> 0) & 0x03,
        })
    }

    pub fn encode(&self) -> Vec<u8> {
        vec![
            (((self.priority & 0x07) << 5) | ((self.channel & 0x07) << 2) | ((self.direction & 0x03) << 0)) as u8,
        ]
    }
}
