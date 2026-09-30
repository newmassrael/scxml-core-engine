// SCE-GENERATED — DO NOT EDIT
// source-hash: 21a662d27007dd95ae857cda1ce2fb7e0fcfe414050da26f9854337c37d1b7b8
// SCE-MAP: algorithm_days_in_month.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.days_in_month`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.days_in_month

fun daysInMonth(year: UShort, month: UByte): UByte {
    var days: UByte = 31.toUByte()
    if (month == 4.toUByte() || month == 6.toUByte() || month == 9.toUByte() || month == 11.toUByte()) {
        days = 30.toUByte()
    }
    if (month == 2.toUByte()) {
        days = 28.toUByte()
        if ((year.toUInt() % 4.toUInt()).toUShort() == 0.toUShort() && (year.toUInt() % 100.toUInt()).toUShort() != 0.toUShort() || (year.toUInt() % 400.toUInt()).toUShort() == 0.toUShort()) {
            days = 29.toUByte()
        }
    }
    return days
}
