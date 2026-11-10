// SCE-GENERATED — DO NOT EDIT
// source-hash: 43faded87b84ec06f387d0e3d6ccb626e2fa1802d2395c65b70588a4c3c394af
#![doc = "SCE-MAP: algorithm_days_in_month.scxml:14 :: _forge_body"]
// SCE-MAP: algorithm_days_in_month.scxml:14 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn month_length(year: u16, month: u8) -> u8 {
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
