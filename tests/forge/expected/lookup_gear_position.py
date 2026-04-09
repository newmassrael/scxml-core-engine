# SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
# Do not edit — regenerate from the source SCXML file.

from enum import Enum


class Gear(Enum):
    PARK = "PARK"
    REVERSE = "REVERSE"
    NEUTRAL = "NEUTRAL"
    DRIVE = "DRIVE"
    SPORT = "SPORT"


def lookup_gear(gear_raw: int) -> Gear:
    if gear_raw == 3:
        return Gear.DRIVE
    elif gear_raw == 2:
        return Gear.NEUTRAL
    elif gear_raw == 0:
        return Gear.PARK
    elif gear_raw == 1:
        return Gear.REVERSE
    elif gear_raw == 4:
        return Gear.SPORT
    else:
        return Gear.NEUTRAL