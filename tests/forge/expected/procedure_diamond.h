// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Do not edit — regenerate from the source SCXML file.
//
// Event-driven state machine using StaticExecutionEngine.
// Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
// Pure decision trees (no events/sends) execute via Event::NONE transitions.

#pragma once
#ifndef SCE_FORGE_PROCEDURE_DIAMOND_L2_H
#define SCE_FORGE_PROCEDURE_DIAMOND_L2_H

#include <cstdint>
#include <functional>
#include <map>
#include <optional>
#include <string>
#include <vector>
#include "static/StaticExecutionEngine.h"
#include "sce/forge/ProcedureServiceTypes.h"

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

// ── State Policy ─────────────────────────────────────────────────

struct ProcedureDiamondPolicy {
    using State = ::SCE::Generated::ProcedureDiamond::State;
    using Event = ::SCE::Generated::ProcedureDiamond::Event;

    static constexpr bool HAS_PARALLEL_STATES = false;
    static constexpr bool NEEDS_SCRIPT_ENGINE = false;

    // ── Datamodel members ────────────────────────────────────────
    uint16_t sensorValue_{};
    std::string mode_{};

    // ── Service handler (for <send sce:service>) ─────────────────
    SCE::Forge::ProcedureServiceHandler serviceHandler_;

    // ── Done data storage ────────────────────────────────────────
    mutable std::map<std::string, std::string> doneData_;

    // ── Event metadata (populated by engine via populatePolicyFromMetadata) ──
    // W3C SCXML 5.10: _event.data binding — written by engine's processEventQueues()
    mutable std::string pendingEventData_;

    // ── Transition tracking (required by StaticExecutionEngine) ──
    mutable State lastTransitionSourceState_{};
    mutable bool lastTransitionIsInternal_ = false;
    mutable bool lastTransitionIsTargetless_ = false;
    mutable size_t lastTransitionIndex_ = 0;
    mutable bool hasTransitionActions_ = false;

    ProcedureDiamondPolicy() = default;

    // ── Hierarchy (flat — no parent states) ──────────────────────

    [[nodiscard]] static constexpr State initialState() noexcept {
        return State::Classify;
    }

    [[nodiscard]] static constexpr bool isFinalState(State state) noexcept {
        switch (state) {
            case State::Accept: return true;
            case State::Reject: return true;
            default: return false;
        }
    }

    [[nodiscard]] static constexpr std::optional<State> getParent([[maybe_unused]] State state) noexcept {
        return std::nullopt;
    }

    [[nodiscard]] static constexpr bool isCompoundState([[maybe_unused]] State state) noexcept {
        return false;
    }

    static State getInitialChild(State state) { return state; }

    static std::vector<State> getInitialChildren(State state) { return {state}; }

    State getInitialOrHistoryChild(State state) const { return state; }

    // ── Event name conversion ────────────────────────────────────

    [[nodiscard]] static constexpr const char* getEventName(Event event) noexcept {
        switch (event) {
            case Event::NONE: return "";
            case Event::Fail: return "fail";
            case Event::Ok: return "ok";
            default: return "";
        }
    }

    [[nodiscard]] static std::optional<Event> getEventFromName(const std::string& name) noexcept {
        if (name.empty()) return std::nullopt;
        if (name == "fail") return Event::Fail;
        if (name == "ok") return Event::Ok;
        return std::nullopt;
    }

    // ── Entry actions (service sends + done data) ────────────────

    template<typename Engine>
    void executeEntryActions(State state, Engine& engine) {
        switch (state) {
            default:
                break;
        }
    }

    // ── Exit actions (none for procedures) ───────────────────────

    template<typename Engine>
    static void executeExitActions([[maybe_unused]] State state, [[maybe_unused]] Engine& engine,
                                   [[maybe_unused]] const std::vector<State>& activeStatesBeforeTransition) {
    }

    // ── Transition processing ────────────────────────────────────

    template<typename Engine>
    bool processTransition(State& currentState, Event event, [[maybe_unused]] Engine& engine) {
        bool transitionTaken = false;

        switch (currentState) {
            case State::Classify:
                // Eventless transition (guard: sensorValue_ > 1000)
                if (event == Event::NONE) {
                    if (sensorValue_ > 1000) {
                        lastTransitionSourceState_ = currentState;
                        lastTransitionIsInternal_ = false;
                        lastTransitionIsTargetless_ = false;
                        lastTransitionIndex_ = 0;
                        hasTransitionActions_ = false;
                        currentState = State::HighPath;
                        transitionTaken = true;
                    }
                }
                if (transitionTaken) return true;
                // Eventless transition (guard: sensorValue_ > 500)
                if (event == Event::NONE) {
                    if (sensorValue_ > 500) {
                        lastTransitionSourceState_ = currentState;
                        lastTransitionIsInternal_ = false;
                        lastTransitionIsTargetless_ = false;
                        lastTransitionIndex_ = 1;
                        hasTransitionActions_ = false;
                        currentState = State::MidPath;
                        transitionTaken = true;
                    }
                }
                if (transitionTaken) return true;
                // Eventless transition
                if (event == Event::NONE) {
                    lastTransitionSourceState_ = currentState;
                    lastTransitionIsInternal_ = false;
                    lastTransitionIsTargetless_ = false;
                    lastTransitionIndex_ = 2;
                    hasTransitionActions_ = false;
                    currentState = State::LowPath;
                    transitionTaken = true;
                }
                if (transitionTaken) return true;
                return false;
            case State::HighPath:
                // Eventless transition (guard: mode_ == "strict")
                if (event == Event::NONE) {
                    if (mode_ == "strict") {
                        lastTransitionSourceState_ = currentState;
                        lastTransitionIsInternal_ = false;
                        lastTransitionIsTargetless_ = false;
                        lastTransitionIndex_ = 0;
                        hasTransitionActions_ = false;
                        currentState = State::Reject;
                        transitionTaken = true;
                    }
                }
                if (transitionTaken) return true;
                // Eventless transition
                if (event == Event::NONE) {
                    lastTransitionSourceState_ = currentState;
                    lastTransitionIsInternal_ = false;
                    lastTransitionIsTargetless_ = false;
                    lastTransitionIndex_ = 1;
                    hasTransitionActions_ = false;
                    currentState = State::Accept;
                    transitionTaken = true;
                }
                if (transitionTaken) return true;
                return false;
            case State::MidPath:
                // Eventless transition
                if (event == Event::NONE) {
                    lastTransitionSourceState_ = currentState;
                    lastTransitionIsInternal_ = false;
                    lastTransitionIsTargetless_ = false;
                    lastTransitionIndex_ = 0;
                    hasTransitionActions_ = false;
                    currentState = State::Accept;
                    transitionTaken = true;
                }
                if (transitionTaken) return true;
                return false;
            case State::LowPath:
                // Eventless transition
                if (event == Event::NONE) {
                    lastTransitionSourceState_ = currentState;
                    lastTransitionIsInternal_ = false;
                    lastTransitionIsTargetless_ = false;
                    lastTransitionIndex_ = 0;
                    hasTransitionActions_ = false;
                    currentState = State::Accept;
                    transitionTaken = true;
                }
                if (transitionTaken) return true;
                return false;
            default:
                return false;
        }
    }

    // ── Transition actions (<assign> in transitions) ─────────────

    template<typename Engine>
    void executeTransitionActions([[maybe_unused]] Engine& engine) {
        if (!hasTransitionActions_) return;
        hasTransitionActions_ = false;
    }

};

// ── State machine class ──────────────────────────────────────────

class ProcedureDiamond : public ::SCE::Static::StaticExecutionEngine<ProcedureDiamondPolicy> {
public:
    using PolicyType = ProcedureDiamondPolicy;
    using Result = SCE::Forge::ProcedureRunResult;

    ProcedureDiamond() = default;

    /// Set the service handler for <send sce:service> actions.
    void setServiceHandler(SCE::Forge::ProcedureServiceHandler handler) {
        getPolicy().serviceHandler_ = std::move(handler);
    }

    /// Set input parameters before calling runToCompletion().
    void setSensorValue(uint16_t value) {
        getPolicy().sensorValue_ = value;
    }
    void setMode(const std::string& value) {
        getPolicy().mode_ = value;
    }

    /// Run the procedure to completion (blocking).
    /// Drives the state machine from initial state through service sends
    /// until a <final> state is reached.
    Result runToCompletion() {
        initialize();
        Result result;
        result.completed = isInFinalState();
        if (result.completed) {
            // Extract state name from enum
            switch (getCurrentState()) {
                case State::Accept: result.final_state = "accept"; break;
                case State::Reject: result.final_state = "reject"; break;
                default: break;
            }
            result.done_data = getPolicy().doneData_;
        }
        return result;
    }
};

// ── Convenience wrapper function ─────────────────────────────────

inline ProcedureDiamond::Result execute(
    SCE::Forge::ProcedureServiceHandler handler,
    uint16_t sensorValue,
    const std::string& mode) {
    ProcedureDiamond sm;
    sm.setServiceHandler(std::move(handler));
    sm.setSensorValue(sensorValue);
    sm.setMode(mode);
    return sm.runToCompletion();
}

// ── Compile-time verification ────────────────────────────────────
#if __cpp_concepts >= 202002L
static_assert(::SCE::Core::EventNamingPolicy<ProcedureDiamondPolicy>,
    "Generated ProcedureDiamondPolicy must satisfy EventNamingPolicy concept");
#endif

}  // namespace SCE::Generated::ProcedureDiamond

#endif  // SCE_FORGE_PROCEDURE_DIAMOND_L2_H
