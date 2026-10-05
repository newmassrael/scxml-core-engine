// SCE-MAP: crossfile_observer_condition:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="observer")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CROSSFILE_OBSERVER_CONDITION_H
#define SCE_FORGE_CROSSFILE_OBSERVER_CONDITION_H

#include <cstdint>
#include <sce/forge/observer.h>
#include "condition_threshold.h"

namespace SCE::Generated::CrossfileObserverCondition {

// No sce:event-domain declared on this <scxml> root: the observer falls back
// to a file-local domain. The resulting Event<> type cannot be composed with
// other observers. To enable cross-file composition, add
// sce:event-domain="..." to the source SCXML. See SCE_FORGE.md §4.11.
struct ForgeDomain {
    enum Tag {
        RAISE_ALARM,
        CLEAR_ALARM
    };
};

class CrossfileObserverCondition {
public:
    SCE::Forge::EventQueue<ForgeDomain> update(double coolantTemp, double oilTemp) {
        SCE::Forge::EventQueue<ForgeDomain> sce_events;
        if (alarm_.enterIf(SCE::Generated::ConditionThreshold::conditionThreshold(coolantTemp, oilTemp, 110.0))) {
            sce_events.push(ForgeDomain::RAISE_ALARM);
        }
        else if (alarm_.leaveIf(SCE::Generated::ConditionThreshold::conditionThreshold(coolantTemp, oilTemp, 120.0) == false)) {
            sce_events.push(ForgeDomain::CLEAR_ALARM);
        }
        return sce_events;
    }

private:
    SCE::Forge::ThresholdState alarm_;
};

}  // namespace SCE::Generated::CrossfileObserverCondition

#endif  // SCE_FORGE_CROSSFILE_OBSERVER_CONDITION_H
