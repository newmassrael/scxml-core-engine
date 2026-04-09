// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Do not edit — regenerate from the source SCXML file.

#![allow(dead_code)]

#[derive(Debug)]
pub struct ProcedureResult {
    pub completed: bool,
    pub final_state: String,
}

const STATE_NAMES: [&str; 6] = ["classify", "high_path", "mid_path", "low_path", "accept", "reject"];

pub fn execute(sensor_value: u16, mode: &str) -> ProcedureResult {
    let mut current: i32 = 0;
    let mut iterations = 0;
    while iterations < 6 {
        iterations += 1;
        let next: i32 = match current {
            0 => {
                if sensor_value > 1000 { 1 }
                else if sensor_value > 500 { 2 }
                else { 3 }
            }
            1 => {
                if mode == "strict" { 5 }
                else { 4 }
            }
            2 => {
                4
            }
            3 => {
                4
            }
            _ => -1,
        };
        if next < 0 { break; }
        current = next;
        if current == 4 || current == 5 { break; }
    }
    let completed = current == 4 || current == 5;
    ProcedureResult {
        completed,
        final_state: STATE_NAMES[current as usize].to_string(),
    }
}