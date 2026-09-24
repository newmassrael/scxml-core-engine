// SCE-GENERATED — DO NOT EDIT
// source-hash: 9682ba42436be01ddeb48458c1e76a0d9251bd3f706e09da9977af19a7844382
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
