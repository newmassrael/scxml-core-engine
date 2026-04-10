// SCE Forge: Auto-generated from Extended SCXML (sce:kind="observer")
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_OBSERVER_COOLANT_H
#define SCE_FORGE_OBSERVER_COOLANT_H

#include <cstdint>
#include <vector>

namespace SCE::Generated::ObserverCoolant {

enum class Event {
    EMIT_WARNING,
    CLEAR_WARNING,
    EMERGENCY_SHUTDOWN
};

using Events = std::vector<Event>;

struct ObserverCoolant {
    bool warningActive_ = false;
    bool criticalActive_ = false;

    Events update(double coolantTemp) {
        Events events;
        if (!warningActive_ && (coolantTemp > 110.0)) {
            warningActive_ = true;
            events.push_back(Event::EMIT_WARNING);
        } else if (warningActive_ && (coolantTemp < 100.0)) {
            warningActive_ = false;
            events.push_back(Event::CLEAR_WARNING);
        }
        if (!criticalActive_ && (coolantTemp > 120.0)) {
            criticalActive_ = true;
            events.push_back(Event::EMERGENCY_SHUTDOWN);
        } else if (criticalActive_ && (coolantTemp < 105.0)) {
            criticalActive_ = false;
        }
        return events;
    }
};

}  // namespace SCE::Generated::ObserverCoolant

#endif  // SCE_FORGE_OBSERVER_COOLANT_H