// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.lookup_gear_position

enum class Gear { PARK, REVERSE, NEUTRAL, DRIVE, SPORT }

fun lookupGear(gearRaw: UByte): Gear = when (gearRaw.toInt()) {
    3 -> Gear.DRIVE
    2 -> Gear.NEUTRAL
    0 -> Gear.PARK
    1 -> Gear.REVERSE
    4 -> Gear.SPORT
    else -> Gear.NEUTRAL
}
