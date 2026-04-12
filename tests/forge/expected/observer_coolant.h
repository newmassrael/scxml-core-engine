// SCE Forge: Auto-generated from Extended SCXML (sce:kind="observer")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_OBSERVER_COOLANT_H
#define SCE_FORGE_OBSERVER_COOLANT_H

#include <cstdint>
#include <sce/forge/observer.h>

namespace SCE::Generated::ObserverCoolant {

// No sce:event-domain declared on this <scxml> root: the observer falls back
// to a file-local domain. The resulting Event<> type cannot be composed with
// other observers. To enable cross-file composition, add
// sce:event-domain="..." to the source SCXML. See SCE_FORGE.md §4.11.
struct ForgeDomain {
    enum Tag {
        EMIT_WARNING,
        CLEAR_WARNING,
        EMERGENCY_SHUTDOWN
    };
};

class ObserverCoolant {
public:
    SCE::Forge::EventQueue<ForgeDomain> update(double coolantTemp) {
        SCE::Forge::EventQueue<ForgeDomain> events;
        if (warning_.enterIf(coolantTemp > 110.0)) {
            events.push(ForgeDomain::EMIT_WARNING);
        }
        else if (warning_.leaveIf(coolantTemp < 100.0)) {
            events.push(ForgeDomain::CLEAR_WARNING);
        }
        if (critical_.enterIf(coolantTemp > 120.0)) {
            events.push(ForgeDomain::EMERGENCY_SHUTDOWN);
        }
        else {
            critical_.leaveIf(coolantTemp < 105.0);
        }
        return events;
    }

private:
    SCE::Forge::ThresholdState warning_;
    SCE::Forge::ThresholdState critical_;
};

}  // namespace SCE::Generated::ObserverCoolant

#endif  // SCE_FORGE_OBSERVER_COOLANT_H
