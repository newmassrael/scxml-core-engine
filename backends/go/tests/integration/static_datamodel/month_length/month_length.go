// SCE-GENERATED — DO NOT EDIT
// source-hash: e939b2700c7d391ee77811c0c5056f1bd75fa31a962245a8882cc047f3a138a0
// SCE-MAP: algorithm_days_in_month.scxml:14 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `month_length`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package month_length

func MonthLength(year uint16, month uint8) uint8 {
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
