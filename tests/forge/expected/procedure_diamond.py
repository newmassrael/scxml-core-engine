# SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
# Do not edit — regenerate from the source SCXML file.

from dataclasses import dataclass


@dataclass
class ProcedureResult:
    completed: bool
    final_state: str


_STATE_NAMES = ["classify", "high_path", "mid_path", "low_path", "accept", "reject"]


def execute(sensor_value: int, mode: str) -> ProcedureResult:
    current = 0
    for _ in range(6):
        if current == 0:
            if sensor_value > 1000:
                current = 1
            elif sensor_value > 500:
                current = 2
            else:
                current = 3
        elif current == 1:
            if mode == 'strict':
                current = 5
            else:
                current = 4
        elif current == 2:
            current = 4
        elif current == 3:
            current = 4
        else:
            break
        if current == 4 or current == 5:
            break
    completed = current == 4 or current == 5
    return ProcedureResult(completed, _STATE_NAMES[current])