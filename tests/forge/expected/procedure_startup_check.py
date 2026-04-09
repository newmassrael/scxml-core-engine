# SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
# Do not edit — regenerate from the source SCXML file.

from dataclasses import dataclass


@dataclass
class ProcedureResult:
    completed: bool
    final_state: str


_STATE_NAMES = ["check_voltage", "check_temp", "success", "fail_voltage", "fail_overtemp"]


def execute(voltage: float, temperature: float) -> ProcedureResult:
    current = 0
    for _ in range(5):
        if current == 0:
            if voltage >= 11.5 and voltage <= 14.5:
                current = 1
            else:
                current = 3
        elif current == 1:
            if temperature < 80.0:
                current = 2
            else:
                current = 4
        else:
            break
        if current == 2 or current == 3 or current == 4:
            break
    completed = current == 2 or current == 3 or current == 4
    return ProcedureResult(completed, _STATE_NAMES[current])