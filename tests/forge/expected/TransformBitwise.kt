// SCE-MAP: transform_bitwise:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.transform_bitwise

fun computeHighNibble(byte: UByte): UByte =
    (byte.toInt() shr 4 and 0x0F).toUByte()

fun computeLowNibble(byte: UByte): UByte =
    (byte.toInt() and 0x0F).toUByte()
