// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_PROCEDURE_LINEAR_H
#define SCE_FORGE_PROCEDURE_LINEAR_H

#include <cstdint>
#include <string>
#include "sce/forge/ProcedureStateMachine.h"

namespace SCE::Generated::ProcedureLinear {

struct ProcedureLinear : public SCE::Forge::ProcedureStateMachine<ProcedureLinear> {
    enum State : int {
        STAGE_A = 0,
        STAGE_B = 1,
        STAGE_C = 2,
        DONE = 3,
    };

    static constexpr int INITIAL_STATE = 0;
    static constexpr int STATE_COUNT = 4;

    static constexpr bool isFinal(int s) {
        return s == 3;
    }

    static const char* stateName(int s) {
        static const char* names[] = { "stage_a", "stage_b", "stage_c", "done" };
        return (s >= 0 && s < STATE_COUNT) ? names[s] : "";
    }

    int evaluateTransitions(int current, int32_t value) const {
        switch (current) {
            case 0:
                return 1;
            case 1:
                return 2;
            case 2:
                return 3;
        }
        return -1;
    }
};

}  // namespace SCE::Generated::ProcedureLinear

#endif  // SCE_FORGE_PROCEDURE_LINEAR_H