// SCE-GENERATED — DO NOT EDIT
// source-hash: ffcd62fbf2cc0765f1d35fa7761f3e9b8facb7dcea254a9b3f1d643e797173c6
// SCE-MAP: algorithm_days_in_month.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `days_in_month`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package days_in_month

func DaysInMonth(year uint16, month uint8) uint8 {
    var days uint8 = 31
    if (month == 4 || month == 6 || month == 9 || month == 11) {
        days = 30;
    }
    if (month == 2) {
        days = 28;
        if (year % 4 == 0 && year % 100 != 0 || year % 400 == 0) {
            days = 29;
        }
    }
    return days
}
