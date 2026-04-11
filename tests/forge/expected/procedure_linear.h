// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Do not edit — regenerate from the source SCXML file.
//
// Event-driven state machine using StaticExecutionEngine.
// Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
// Pure decision trees (no events/sends) execute via Event::NONE transitions.

#pragma once
#ifndef SCE_FORGE_PROCEDURE_LINEAR_L2_H
#define SCE_FORGE_PROCEDURE_LINEAR_L2_H

#include <cstdint>
#include <functional>
#include <map>
#include <optional>
#include <string>
#include <vector>
#include "static/StaticExecutionEngine.h"
#include "sce/forge/ProcedureServiceTypes.h"

namespace SCE::Generated::ProcedureLinear {

// ── State and Event enums ────────────────────────────────────────

enum class State : uint8_t {
    StageA = 0,
    StageB = 1,
    StageC = 2,
    Done = 3
};

enum class Event : uint8_t {
    NONE = 0,
    Fail = 1,
    Ok = 2
};

// ── State Policy ─────────────────────────────────────────────────

struct ProcedureLinearPolicy {
    using State = ::SCE::Generated::ProcedureLinear::State;
    using Event = ::SCE::Generated::ProcedureLinear::Event;

    static constexpr bool HAS_PARALLEL_STATES = false;
    static constexpr bool NEEDS_SCRIPT_ENGINE = false;

    // ── Datamodel members ────────────────────────────────────────
    int32_t value_{};

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

    ProcedureLinearPolicy() = default;

    // ── Hierarchy (flat — no parent states) ──────────────────────

    [[nodiscard]] static constexpr State initialState() noexcept {
        return State::StageA;
    }

    [[nodiscard]] static constexpr bool isFinalState(State state) noexcept {
        switch (state) {
            case State::Done: return true;
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
            case State::StageA:
                // Eventless transition
                if (event == Event::NONE) {
                    lastTransitionSourceState_ = currentState;
                    lastTransitionIsInternal_ = false;
                    lastTransitionIsTargetless_ = false;
                    lastTransitionIndex_ = 0;
                    hasTransitionActions_ = false;
                    currentState = State::StageB;
                    transitionTaken = true;
                }
                if (transitionTaken) return true;
                return false;
            case State::StageB:
                // Eventless transition
                if (event == Event::NONE) {
                    lastTransitionSourceState_ = currentState;
                    lastTransitionIsInternal_ = false;
                    lastTransitionIsTargetless_ = false;
                    lastTransitionIndex_ = 0;
                    hasTransitionActions_ = false;
                    currentState = State::StageC;
                    transitionTaken = true;
                }
                if (transitionTaken) return true;
                return false;
            case State::StageC:
                // Eventless transition
                if (event == Event::NONE) {
                    lastTransitionSourceState_ = currentState;
                    lastTransitionIsInternal_ = false;
                    lastTransitionIsTargetless_ = false;
                    lastTransitionIndex_ = 0;
                    hasTransitionActions_ = false;
                    currentState = State::Done;
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

class ProcedureLinear : public ::SCE::Static::StaticExecutionEngine<ProcedureLinearPolicy> {
public:
    using PolicyType = ProcedureLinearPolicy;
    using Result = SCE::Forge::ProcedureRunResult;

    ProcedureLinear() = default;

    /// Set the service handler for <send sce:service> actions.
    void setServiceHandler(SCE::Forge::ProcedureServiceHandler handler) {
        getPolicy().serviceHandler_ = std::move(handler);
    }

    /// Set input parameters before calling runToCompletion().
    void setValue(int32_t value) {
        getPolicy().value_ = value;
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
                case State::Done: result.final_state = "done"; break;
                default: break;
            }
            result.done_data = getPolicy().doneData_;
        }
        return result;
    }
};

// ── Convenience wrapper function ─────────────────────────────────

inline ProcedureLinear::Result execute(
    SCE::Forge::ProcedureServiceHandler handler,
    int32_t value) {
    ProcedureLinear sm;
    sm.setServiceHandler(std::move(handler));
    sm.setValue(value);
    return sm.runToCompletion();
}

// ── Compile-time verification ────────────────────────────────────
#if __cpp_concepts >= 202002L
static_assert(::SCE::Core::EventNamingPolicy<ProcedureLinearPolicy>,
    "Generated ProcedureLinearPolicy must satisfy EventNamingPolicy concept");
#endif

}  // namespace SCE::Generated::ProcedureLinear

#endif  // SCE_FORGE_PROCEDURE_LINEAR_L2_H
