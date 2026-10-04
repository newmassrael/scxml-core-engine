// SCE-GENERATED — DO NOT EDIT
// source-hash: c769688680043642d7b46211c31312f7471f398adc2bffecf5e2b210fa2f7270
#![doc = "SCE-MAP: algorithm_days_in_month.scxml:8 :: _forge_body"]
// SCE-MAP: algorithm_days_in_month.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn days_in_month(year: u16, month: u8) -> u8 {
    let mut days: u8 = 31;
    if month == 4 || month == 6 || month == 9 || month == 11 {
        days = 30;
    }
    if month == 2 {
        days = 28;
        if year % 4 == 0 && year % 100 != 0 || year % 400 == 0 {
            days = 29;
        }
    }
    return days;
}
