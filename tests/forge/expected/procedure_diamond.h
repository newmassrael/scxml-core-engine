// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_PROCEDURE_DIAMOND_H
#define SCE_FORGE_PROCEDURE_DIAMOND_H

#include <cstdint>
#include <string>
#include "core/ProcedureStateMachine.h"

namespace SCE::Generated::ProcedureDiamond {

struct ProcedureDiamond : public SCE::Core::ProcedureStateMachine<ProcedureDiamond> {
    enum State : int {
        CLASSIFY = 0,
        HIGH_PATH = 1,
        MID_PATH = 2,
        LOW_PATH = 3,
        ACCEPT = 4,
        REJECT = 5,
    };

    static constexpr int INITIAL_STATE = 0;
    static constexpr int STATE_COUNT = 6;

    static constexpr bool isFinal(int s) {
        return s == 4 || s == 5;
    }

    static const char* stateName(int s) {
        static const char* names[] = { "classify", "high_path", "mid_path", "low_path", "accept", "reject" };
        return (s >= 0 && s < STATE_COUNT) ? names[s] : "";
    }

    int evaluateTransitions(int current, uint16_t sensorValue, const std::string& mode) const {
        switch (current) {
            case 0:
                if (sensorValue > 1000) return 1;
                if (sensorValue > 500) return 2;
                return 3;
            case 1:
                if (mode == "strict") return 5;
                return 4;
            case 2:
                return 4;
            case 3:
                return 4;
        }
        return -1;
    }
};

}  // namespace SCE::Generated::ProcedureDiamond

#endif  // SCE_FORGE_PROCEDURE_DIAMOND_H