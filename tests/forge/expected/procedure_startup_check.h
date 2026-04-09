// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_PROCEDURE_STARTUP_CHECK_H
#define SCE_FORGE_PROCEDURE_STARTUP_CHECK_H

#include <cstdint>
#include <string>
#include "core/ProcedureStateMachine.h"

namespace SCE::Generated::ProcedureStartupCheck {

struct ProcedureStartupCheck : public SCE::Core::ProcedureStateMachine<ProcedureStartupCheck> {
    enum State : int {
        CHECK_VOLTAGE = 0,
        CHECK_TEMP = 1,
        SUCCESS = 2,
        FAIL_VOLTAGE = 3,
        FAIL_OVERTEMP = 4,
    };

    static constexpr int INITIAL_STATE = 0;
    static constexpr int STATE_COUNT = 5;

    static constexpr bool isFinal(int s) {
        return s == 2 || s == 3 || s == 4;
    }

    static const char* stateName(int s) {
        static const char* names[] = { "check_voltage", "check_temp", "success", "fail_voltage", "fail_overtemp" };
        return (s >= 0 && s < STATE_COUNT) ? names[s] : "";
    }

    int evaluateTransitions(int current, float voltage, float temperature) const {
        switch (current) {
            case 0:
                if (voltage >= 11.5 && voltage <= 14.5) return 1;
                return 3;
            case 1:
                if (temperature < 80.0) return 2;
                return 4;
        }
        return -1;
    }
};

}  // namespace SCE::Generated::ProcedureStartupCheck

#endif  // SCE_FORGE_PROCEDURE_STARTUP_CHECK_H