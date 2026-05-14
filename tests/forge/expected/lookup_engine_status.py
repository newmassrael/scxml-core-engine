# SCE-MAP: lookup_engine_status:3

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from enum import Enum


class Status(Enum):
    STOP = "STOP"
    RUNNING = "RUNNING"
    FAULT = "FAULT"


def lookup_status(eng_sta: int) -> Status:
    if eng_sta == 0x07:
        return Status.FAULT
    elif eng_sta == 0x03:
        return Status.RUNNING
    elif eng_sta in (0x00, 0x01, 0x02):
        return Status.STOP
    else:
        return Status.STOP
