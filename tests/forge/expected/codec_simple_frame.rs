// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Do not edit — regenerate from the source SCXML file.

#![allow(dead_code)]

pub struct CodecSimpleFrame {
    pub msg_id: u8,
    pub length: u8,
    pub payload: u16,
}

impl CodecSimpleFrame {
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