// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure", Level 2)
// Do not edit — regenerate from the source SCXML file.
//
// Level 2 procedure: event-driven state machine using StaticExecutionEngine.
// Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
//
// External dependencies (from sce:payload expressions — must be in scope):
//   frame.encode()

#pragma once
#ifndef SCE_FORGE_CROSSFILE_PROCEDURE_CODEC_L2_H
#define SCE_FORGE_CROSSFILE_PROCEDURE_CODEC_L2_H

#include <cstdint>
#include <functional>
#include <map>
#include <optional>
#include <string>
#include <vector>
#include "static/StaticExecutionEngine.h"
#include "core/ProcedureServiceTypes.h"
#include "codec_simple_frame.h"

namespace SCE::Generated::CrossfileProcedureCodec {

// ── State and Event enums ────────────────────────────────────────

enum class State : uint8_t {
    SendRequest = 0,
    Decode = 1,
    Done = 2,
    Error = 3
};

enum class Event : uint8_t {
    NONE = 0,
    Fail = 1,
    Ok = 2
};

// ── State Policy ─────────────────────────────────────────────────

struct CrossfileProcedureCodecPolicy {
    using State = ::SCE::Generated::CrossfileProcedureCodec::State;
    using Event = ::SCE::Generated::CrossfileProcedureCodec::Event;

    static constexpr bool HAS_PARALLEL_STATES = false;
    static constexpr bool NEEDS_SCRIPT_ENGINE = false;

    // ── Datamodel members ────────────────────────────────────────
    uint32_t ecuAddr_{};
    std::vector<uint8_t> response_;

    // ── Imported kind members (cross-file composition) ──────────
    ::SCE::Generated::CodecSimpleFrame::CodecSimpleFrame frame_{};

    // ── Service handler (for <send sce:service>) ─────────────────
    SCE::Core::ProcedureServiceHandler serviceHandler_;

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

    CrossfileProcedureCodecPolicy() = default;

    // ── Hierarchy (flat — no parent states) ──────────────────────

    [[nodiscard]] static constexpr State initialState() noexcept {
        return State::SendRequest;
    }

    [[nodiscard]] static constexpr bool isFinalState(State state) noexcept {
        switch (state) {
            case State::Done: return true;
            case State::Error: return true;
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
            case State::SendRequest: {
                // <send sce:service="Diag" sce:addr="...">
                if (serviceHandler_) {
                    SCE::Core::ProcedureServiceRequest req;
                    req.service = "Diag";
                    req.params.push_back({"addr", std::to_string(ecuAddr_)});
                    req.params.push_back({"payload", frame_.encode()});
                    auto resp = serviceHandler_(req);
                    engine.raise(typename Engine::EventWithMetadata(
                        resp.success ? Event::Ok : Event::Fail, resp.data));
                }
                break;
            }
            case State::Done: {
                // <donedata>
                doneData_["result"] = "success";
                break;
            }
            case State::Error: {
                // <donedata>
                doneData_["result"] = "failure";
                break;
            }
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
            case State::SendRequest:
                // Event-driven transition: event="ok"
                if (event == Event::Ok) {
                    lastTransitionSourceState_ = currentState;
                    lastTransitionIsInternal_ = false;
                    lastTransitionIsTargetless_ = false;
                    lastTransitionIndex_ = 0;
                    hasTransitionActions_ = true;
                    currentState = State::Decode;
                    transitionTaken = true;
                }
                if (transitionTaken) return true;
                // Event-driven transition: event="fail"
                if (event == Event::Fail) {
                    lastTransitionSourceState_ = currentState;
                    lastTransitionIsInternal_ = false;
                    lastTransitionIsTargetless_ = false;
                    lastTransitionIndex_ = 1;
                    hasTransitionActions_ = false;
                    currentState = State::Error;
                    transitionTaken = true;
                }
                if (transitionTaken) return true;
                return false;
            case State::Decode:
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
        if (lastTransitionSourceState_ == State::SendRequest) {
            if (lastTransitionIndex_ == 0) {
                response_ = std::vector<uint8_t>(pendingEventData_.begin(), pendingEventData_.end());
            }
        }
        hasTransitionActions_ = false;
    }

};

// ── State machine class ──────────────────────────────────────────

class CrossfileProcedureCodec : public ::SCE::Static::StaticExecutionEngine<CrossfileProcedureCodecPolicy> {
public:
    using PolicyType = CrossfileProcedureCodecPolicy;
    using Result = SCE::Core::ProcedureRunResult;

    CrossfileProcedureCodec() = default;

    /// Set the service handler for <send sce:service> actions.
    void setServiceHandler(SCE::Core::ProcedureServiceHandler handler) {
        getPolicy().serviceHandler_ = std::move(handler);
    }

    /// Set input parameters before calling runToCompletion().
    void setEcuAddr(uint32_t value) {
        getPolicy().ecuAddr_ = value;
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
                case State::Error: result.final_state = "error"; break;
                default: break;
            }
            result.done_data = getPolicy().doneData_;
        }
        return result;
    }
};

// ── Convenience wrapper function ─────────────────────────────────

inline CrossfileProcedureCodec::Result executeCrossfileProcedureCodec(
    SCE::Core::ProcedureServiceHandler handler,
    uint32_t ecuAddr) {
    CrossfileProcedureCodec sm;
    sm.setServiceHandler(std::move(handler));
    sm.setEcuAddr(ecuAddr);
    return sm.runToCompletion();
}

// ── Compile-time verification ────────────────────────────────────
#if __cpp_concepts >= 202002L
static_assert(::SCE::Core::EventNamingPolicy<CrossfileProcedureCodecPolicy>,
    "Generated CrossfileProcedureCodecPolicy must satisfy EventNamingPolicy concept");
#endif

}  // namespace SCE::Generated::CrossfileProcedureCodec

#endif  // SCE_FORGE_CROSSFILE_PROCEDURE_CODEC_L2_H