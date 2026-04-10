// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Do not edit — regenerate from the source SCXML file.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Stop,
    Running,
    Fault,
}

pub fn lookup_status(eng_sta: u8) -> Status {
    match eng_sta {
        0x07 => Status::Fault,
        0x03 => Status::Running,
        0x00 | 0x01 | 0x02 => Status::Stop,
        _ => Status::Stop,
    }
}
