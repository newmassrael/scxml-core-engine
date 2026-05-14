// SCE-MAP: procedure_startup_check:2

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// Event-driven state machine driven by SCE::Forge::run_procedure().
// Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
// Pure decision trees (no events/sends) execute via Event::NONE transitions.

#pragma once
#ifndef SCE_FORGE_PROCEDURE_STARTUP_CHECK_L2_H
#define SCE_FORGE_PROCEDURE_STARTUP_CHECK_L2_H

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

namespace SCE::Generated::ProcedureStartupCheck {

// ── State and Event enums ────────────────────────────────────────

enum class State : uint8_t {
    CheckVoltage = 0,
    CheckTemp = 1,
    Success = 2,
    FailVoltage = 3,
    FailOvertemp = 4
};

enum class Event : uint8_t {
    NONE = 0,
    ErrorExecution = 1,
    Fail = 2,
    Ok = 3
};

// ── State machine class ──────────────────────────────────────────

class ProcedureStartupCheck {
public:
    using State = ::SCE::Generated::ProcedureStartupCheck::State;
    using Event = ::SCE::Generated::ProcedureStartupCheck::Event;

    ProcedureStartupCheck() = default;

    // ── Public setters ───────────────────────────────────────────

    /// Set the service handler for <send sce:service> actions.
    void setServiceHandler(SCE::Forge::ProcedureServiceHandler handler) {
        serviceHandler_ = std::move(handler);
    }

    /// Set input parameters before calling runToCompletion().
    void setVoltage(float value) {
        voltage_ = value;
    }
    void setTemperature(float value) {
        temperature_ = value;
    }

    /// Run the procedure to completion (blocking). Delegates to the
    /// shared event loop in sce-forge-runtime/cpp, which mirrors the
    /// return-event shape used by Rust / Python / Kotlin / Go.
    SCE::Forge::ProcedureRunResult runToCompletion() {
        return SCE::Forge::run_procedure(*this);
    }

    // ── Static policy metadata ───────────────────────────────────

    [[nodiscard]] static constexpr State initialState() noexcept {
        return State::CheckVoltage;
    }

    [[nodiscard]] static constexpr Event noneEvent() noexcept {
        return Event::NONE;
    }

    [[nodiscard]] static constexpr bool isFinalState(State state) noexcept {
        switch (state) {
            case State::Success: return true;
            case State::FailVoltage: return true;
            case State::FailOvertemp: return true;
            default: return false;
        }
    }

    [[nodiscard]] static constexpr const char* finalStateName(State state) noexcept {
        switch (state) {
            case State::Success: return "success";
            case State::FailVoltage: return "fail_voltage";
            case State::FailOvertemp: return "fail_overtemp";
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
            case State::CheckVoltage:
                // Eventless transition (guard: voltage_ >= 11.5 && voltage_ <= 14.5)
                if (event == Event::NONE) {
                    if (voltage_ >= 11.5 && voltage_ <= 14.5) {
                        return std::make_tuple(State::CheckTemp, std::size_t{ 0 }, false);
                    }
                }
                // Eventless transition
                if (event == Event::NONE) {
                    return std::make_tuple(State::FailVoltage, std::size_t{ 1 }, false);
                }
                return std::nullopt;
            case State::CheckTemp:
                // Eventless transition (guard: temperature_ < 80.0)
                if (event == Event::NONE) {
                    if (temperature_ < 80.0) {
                        return std::make_tuple(State::Success, std::size_t{ 0 }, false);
                    }
                }
                // Eventless transition
                if (event == Event::NONE) {
                    return std::make_tuple(State::FailOvertemp, std::size_t{ 1 }, false);
                }
                return std::nullopt;
            default:
                return std::nullopt;
        }
    }

    // ── Transition actions (<assign> in transitions) ─────────────
    //
    // Returns std::nullopt for normal flow; std::optional<Event> with a
    // value when an assign-time check (RFC `claudedocs/rfc-forge-bytes-bounded.md`
    // §3 B4 bytes cap violation) raises an internal event. The shared
    // run_procedure() loop re-processes the source state with that event
    // so a fixture's `<transition event="error.execution">` can pick it up.

    [[nodiscard]] std::optional<Event> executeTransitionActions([[maybe_unused]] State source, [[maybe_unused]] std::size_t trIndex) {
        return std::nullopt;
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
    float voltage_{};
    float temperature_{};

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
    float voltage,
    float temperature) {
    ProcedureStartupCheck sm;
    sm.setServiceHandler(std::move(handler));
    sm.setVoltage(voltage);
    sm.setTemperature(temperature);
    return sm.runToCompletion();
}

}  // namespace SCE::Generated::ProcedureStartupCheck

#endif  // SCE_FORGE_PROCEDURE_STARTUP_CHECK_L2_H
