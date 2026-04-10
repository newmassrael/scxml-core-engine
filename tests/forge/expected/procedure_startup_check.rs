// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Do not edit — regenerate from the source SCXML file.

#[derive(Debug)]
pub struct ProcedureResult {
    pub completed: bool,
    pub final_state: String,
}

const STATE_NAMES: [&str; 5] = ["check_voltage", "check_temp", "success", "fail_voltage", "fail_overtemp"];

pub fn execute(voltage: f32, temperature: f32) -> ProcedureResult {
    let mut current: i32 = 0;
    let mut iterations = 0;
    while iterations < 5 {
        iterations += 1;
        let next: i32 = match current {
            0 => {
                if voltage >= 11.5 && voltage <= 14.5 { 1 }
                else { 3 }
            }
            1 => {
                if temperature < 80.0 { 2 }
                else { 4 }
            }
            _ => -1,
        };
        if next < 0 { break; }
        current = next;
        if current == 2 || current == 3 || current == 4 { break; }
    }
    let completed = current == 2 || current == 3 || current == 4;
    ProcedureResult {
        completed,
        final_state: STATE_NAMES[current as usize].to_string(),
    }
}