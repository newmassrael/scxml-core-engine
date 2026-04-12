// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// Event-driven state machine driven by SCE::Forge::run_procedure().
// Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
// Pure decision trees (no events/sends) execute via Event::NONE transitions.

#pragma once
#ifndef SCE_FORGE_PROCEDURE_DIAMOND_L2_H
#define SCE_FORGE_PROCEDURE_DIAMOND_L2_H

#include <cstdint>
#include <cstddef>
#include <map>
#include <optional>
#include <string>
#include <tuple>
#include <utility>
#include <vector>

#include "sce/forge/ProcedureServiceTypes.h"
#include "sce/forge/ProcedureStateMachine.h"

namespace SCE::Generated::ProcedureDiamond {

// ── State and Event enums ────────────────────────────────────────

enum class State : uint8_t {
    Classify = 0,
    HighPath = 1,
    MidPath = 2,
    LowPath = 3,
    Accept = 4,
    Reject = 5
};

enum class Event : uint8_t {
    NONE = 0,
    Fail = 1,
    Ok = 2
};

// ── State machine class ──────────────────────────────────────────

class ProcedureDiamond {
public:
    using State = ::SCE::Generated::ProcedureDiamond::State;
    using Event = ::SCE::Generated::ProcedureDiamond::Event;

    ProcedureDiamond() = default;

    // ── Public setters ───────────────────────────────────────────

    /// Set the service handler for <send sce:service> actions.
    void setServiceHandler(SCE::Forge::ProcedureServiceHandler handler) {
        serviceHandler_ = std::move(handler);
    }

    /// Set input parameters before calling runToCompletion().
    void setSensorValue(uint16_t value) {
        sensorValue_ = value;
    }
    void setMode(const std::string& value) {
        mode_ = value;
    }

    /// Run the procedure to completion (blocking). Delegates to the
    /// shared event loop in sce-forge-runtime/cpp, which mirrors the
    /// return-event shape used by Rust / Python / Kotlin / Go.
    SCE::Forge::ProcedureRunResult runToCompletion() {
        return SCE::Forge::run_procedure(*this);
    }

    // ── Static policy metadata ───────────────────────────────────

    [[nodiscard]] static constexpr State initialState() noexcept {
        return State::Classify;
    }

    [[nodiscard]] static constexpr Event noneEvent() noexcept {
        return Event::NONE;
    }

    [[nodiscard]] static constexpr bool isFinalState(State state) noexcept {
        switch (state) {
            case State::Accept: return true;
            case State::Reject: return true;
            default: return false;
        }
    }

    [[nodiscard]] static constexpr const char* finalStateName(State state) noexcept {
        switch (state) {
            case State::Accept: return "accept";
            case State::Reject: return "reject";
            default: return "";
        }
    }

    // ── Entry actions (service sends + done data) ────────────────
    //
    // Returns the event raised by this state's <send sce:service> (if
    // any) along with its response data string, matching the shape of
    // the Rust / Python / Kotlin / Go runtimes. A state without a send
    // returns (Event::NONE, "") so that the loop falls through to
    // eventless transitions.

    std::pair<Event, std::string> executeEntryActions(State state) {
        switch (state) {
            default:
                break;
        }
        return { Event::NONE, std::string() };
    }

    // ── Transition processing ────────────────────────────────────
    //
    // Returns the next state, transition index, and whether the
    // transition has <assign> actions, or std::nullopt if no
    // transition fires. The caller applies assigns via
    // executeTransitionActions(source, trIndex).

    [[nodiscard]] std::optional<std::tuple<State, std::size_t, bool>>
    processTransition(State state, Event event) const {
        switch (state) {
            case State::Classify:
                // Eventless transition (guard: sensorValue_ > 1000)
                if (event == Event::NONE) {
                    if (sensorValue_ > 1000) {
                        return std::make_tuple(State::HighPath, std::size_t{ 0 }, false);
                    }
                }
                // Eventless transition (guard: sensorValue_ > 500)
                if (event == Event::NONE) {
                    if (sensorValue_ > 500) {
                        return std::make_tuple(State::MidPath, std::size_t{ 1 }, false);
                    }
                }
                // Eventless transition
                if (event == Event::NONE) {
                    return std::make_tuple(State::LowPath, std::size_t{ 2 }, false);
                }
                return std::nullopt;
            case State::HighPath:
                // Eventless transition (guard: mode_ == "strict")
                if (event == Event::NONE) {
                    if (mode_ == "strict") {
                        return std::make_tuple(State::Reject, std::size_t{ 0 }, false);
                    }
                }
                // Eventless transition
                if (event == Event::NONE) {
                    return std::make_tuple(State::Accept, std::size_t{ 1 }, false);
                }
                return std::nullopt;
            case State::MidPath:
                // Eventless transition
                if (event == Event::NONE) {
                    return std::make_tuple(State::Accept, std::size_t{ 0 }, false);
                }
                return std::nullopt;
            case State::LowPath:
                // Eventless transition
                if (event == Event::NONE) {
                    return std::make_tuple(State::Accept, std::size_t{ 0 }, false);
                }
                return std::nullopt;
            default:
                return std::nullopt;
        }
    }

    // ── Transition actions (<assign> in transitions) ─────────────

    void executeTransitionActions([[maybe_unused]] State source, [[maybe_unused]] std::size_t trIndex) {
    }

    // ── Engine-visible datamodel slots (called by run_procedure) ──

    void setPendingEventData(std::string data) {
        pendingEventData_ = std::move(data);
    }

    [[nodiscard]] const std::map<std::string, std::string>& doneData() const {
        return doneData_;
    }

private:
    // ── Datamodel members ────────────────────────────────────────
    uint16_t sensorValue_{};
    std::string mode_{};

    // ── Service handler (for <send sce:service>) ─────────────────
    SCE::Forge::ProcedureServiceHandler serviceHandler_;

    // ── Done data storage ────────────────────────────────────────
    std::map<std::string, std::string> doneData_;

    // ── W3C SCXML 5.10: _event.data binding ──────────────────────
    std::string pendingEventData_;
};

// ── Convenience wrapper function ─────────────────────────────────

inline SCE::Forge::ProcedureRunResult execute(
    SCE::Forge::ProcedureServiceHandler handler,
    uint16_t sensorValue,
    const std::string& mode) {
    ProcedureDiamond sm;
    sm.setServiceHandler(std::move(handler));
    sm.setSensorValue(sensorValue);
    sm.setMode(mode);
    return sm.runToCompletion();
}

}  // namespace SCE::Generated::ProcedureDiamond

#endif  // SCE_FORGE_PROCEDURE_DIAMOND_L2_H
