// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec). The kind-agnostic conformance harness only references a
// subset of fixtures, so unused-but-pub fields/methods would otherwise
// trigger dead_code on every codec build.
#[allow(dead_code)]
#[derive(Default)]
pub struct CodecTail {
    pub msg_id: u8,
    pub status: u8,
    pub payload: Vec<u8>,
}

#[allow(dead_code)]
impl CodecTail {
    /// Construct an instance with every field zero-initialized via
    /// [`Default`]. Generated procedure_l2 code stores codec instances
    /// as owned members and needs an infallible constructor to
    /// initialize them before any `encode()` or `decode()` call.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn decode(raw: &[u8]) -> Option<Self> {
        if raw.len() < 2 {
            return None;
        }
        Some(Self {
            msg_id: raw[0],
            status: raw[1],
            payload: raw[2..].to_vec(),
        })
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut r: Vec<u8> = Vec::with_capacity(34);
        r.push(self.msg_id);
        r.push(self.status);
        r.extend_from_slice(&self.payload);
        r
    }
}
