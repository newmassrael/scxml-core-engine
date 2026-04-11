// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_LOOKUP_GEAR_POSITION_H
#define SCE_FORGE_LOOKUP_GEAR_POSITION_H

#include <cstdint>

namespace SCE::Generated::LookupGearPosition {

enum class Gear { PARK, REVERSE, NEUTRAL, DRIVE, SPORT };

inline Gear lookupGear(uint8_t gearRaw) {
    switch (gearRaw) {
        case 3:
            return Gear::DRIVE;
        case 2:
            return Gear::NEUTRAL;
        case 0:
            return Gear::PARK;
        case 1:
            return Gear::REVERSE;
        case 4:
            return Gear::SPORT;
        default: return Gear::NEUTRAL;
    }
}

}  // namespace SCE::Generated::LookupGearPosition

#endif  // SCE_FORGE_LOOKUP_GEAR_POSITION_H
