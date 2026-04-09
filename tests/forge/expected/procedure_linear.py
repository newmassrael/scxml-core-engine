# SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
# Do not edit — regenerate from the source SCXML file.

from dataclasses import dataclass


@dataclass
class ProcedureResult:
    completed: bool
    final_state: str


_STATE_NAMES = ["stage_a", "stage_b", "stage_c", "done"]


def execute(value: int) -> ProcedureResult:
    current = 0
    for _ in range(4):
        if current == 0:
            current = 1
        elif current == 1:
            current = 2
        elif current == 2:
            current = 3
        else:
            break
        if current == 3:
            break
    completed = current == 3
    return ProcedureResult(completed, _STATE_NAMES[current])