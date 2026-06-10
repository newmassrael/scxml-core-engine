// SCE-MAP: procedure_security_access:1

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// Event-driven state machine driven by SCE::Forge::run_procedure().
// Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
// Pure decision trees (no events/sends) execute via Event::NONE transitions.
//
// External dependencies (from sce:payload expressions — must be in scope):
//   computeKey(seed)

#pragma once
#ifndef SCE_FORGE_PROCEDURE_SECURITY_ACCESS_L2_H
#define SCE_FORGE_PROCEDURE_SECURITY_ACCESS_L2_H

#include <cstdint>
#include <cstddef>
#include <functional>
#include <stdexcept>
#include <map>
#include <optional>
#include <string>
#include <tuple>
#include <utility>
#include <vector>

#include "sce/forge/ProcedureServiceTypes.h"
#include "sce/forge/ProcedureStateMachine.h"

namespace SCE::Generated::ProcedureSecurityAccess {

// ── State and Event enums ────────────────────────────────────────

enum class State : uint8_t {
    SendTesterPresent = 0,
    RequestSeed = 1,
    SendKey = 2,
    Retry = 3,
    Done = 4,
    Error = 5
};

enum class Event : uint8_t {
    NONE = 0,
    ErrorExecution = 1,
    Fail = 2,
    Ok = 3
};

// ── State machine class ──────────────────────────────────────────

class ProcedureSecurityAccess {
public:
    using State = ::SCE::Generated::ProcedureSecurityAccess::State;
    using Event = ::SCE::Generated::ProcedureSecurityAccess::Event;

    ProcedureSecurityAccess() = default;

    // ── Public setters ───────────────────────────────────────────

    /// Set the service handler for <send sce:service> actions.
    void setServiceHandler(SCE::Forge::ProcedureServiceHandler handler) {
        serviceHandler_ = std::move(handler);
    }

    /// Set the <sce:helper name="computeKey"> DI closure.
    void setComputeKey(std::function<std::vector<uint8_t>(const std::vector<uint8_t>&)> fn) {
        computeKey_ = std::move(fn);
    }

    /// Set input parameters before calling runToCompletion().
    void setEcuAddr(uint32_t value) {
        ecuAddr_ = value;
    }

    /// Run the procedure to completion (blocking). Delegates to the
    /// shared event loop in sce-forge-runtime/cpp, which mirrors the
    /// return-event shape used by Rust / Python / Kotlin / Go.
    SCE::Forge::ProcedureRunResult runToCompletion() {
        return SCE::Forge::run_procedure(*this);
    }

    // ── Static policy metadata ───────────────────────────────────

    [[nodiscard]] static constexpr State initialState() noexcept {
        return State::SendTesterPresent;
    }

    [[nodiscard]] static constexpr Event noneEvent() noexcept {
        return Event::NONE;
    }

    [[nodiscard]] static constexpr bool isFinalState(State state) noexcept {
        switch (state) {
            case State::Done: return true;
            case State::Error: return true;
            default: return false;
        }
    }

    [[nodiscard]] static constexpr const char* finalStateName(State state) noexcept {
        switch (state) {
            case State::Done: return "done";
            case State::Error: return "error";
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
            case State::SendTesterPresent: {
                // <send sce:service="TesterPresent" sce:addr="...">
                if (serviceHandler_) {
                    SCE::Forge::ProcedureServiceRequest req;
                    req.service = "TesterPresent";
                    req.addr = std::to_string(ecuAddr_);
                    auto resp = serviceHandler_(req);
                    return { resp.success ? Event::Ok : Event::Fail, resp.data };
                }
                break;
            }
            case State::RequestSeed: {
                // <send sce:service="SecurityAccess" sce:subfunc="0x01">
                if (serviceHandler_) {
                    SCE::Forge::ProcedureServiceRequest req;
                    req.service = "SecurityAccess";
                    req.subfunc = std::string("0x01");
                    auto resp = serviceHandler_(req);
                    return { resp.success ? Event::Ok : Event::Fail, resp.data };
                }
                break;
            }
            case State::SendKey: {
                // <send sce:service="SecurityAccess" sce:subfunc="0x02">
                if (serviceHandler_) {
                    SCE::Forge::ProcedureServiceRequest req;
                    req.service = "SecurityAccess";
                    req.subfunc = std::string("0x02");
                    req.payload = computeKey_(seed_);
                    auto resp = serviceHandler_(req);
                    return { resp.success ? Event::Ok : Event::Fail, resp.data };
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
            case State::SendTesterPresent:
                // Event-driven transition: event="ok"
                if (event == Event::Ok) {
                    return std::make_tuple(State::RequestSeed, std::size_t{ 0 }, false);
                }
                // Event-driven transition: event="fail"
                if (event == Event::Fail) {
                    return std::make_tuple(State::Error, std::size_t{ 1 }, false);
                }
                return std::nullopt;
            case State::RequestSeed:
                // Event-driven transition: event="ok"
                if (event == Event::Ok) {
                    return std::make_tuple(State::SendKey, std::size_t{ 0 }, true);
                }
                // Event-driven transition: event="fail"
                if (event == Event::Fail) {
                    return std::make_tuple(State::Retry, std::size_t{ 1 }, false);
                }
                return std::nullopt;
            case State::SendKey:
                // Event-driven transition: event="ok"
                if (event == Event::Ok) {
                    return std::make_tuple(State::Done, std::size_t{ 0 }, false);
                }
                // Event-driven transition: event="fail"
                if (event == Event::Fail) {
                    return std::make_tuple(State::Retry, std::size_t{ 1 }, false);
                }
                return std::nullopt;
            case State::Retry:
                // Eventless transition (guard: retryCount_ < maxRetries_)
                if (event == Event::NONE) {
                    if (retryCount_ < maxRetries_) {
                        return std::make_tuple(State::RequestSeed, std::size_t{ 0 }, true);
                    }
                }
                // Eventless transition (guard: retryCount_ >= maxRetries_)
                if (event == Event::NONE) {
                    if (retryCount_ >= maxRetries_) {
                        return std::make_tuple(State::Error, std::size_t{ 1 }, false);
                    }
                }
                return std::nullopt;
            default:
                return std::nullopt;
        }
    }

    // ── Transition actions (<assign> in transitions) ─────────────
    //
    // Returns std::nullopt for normal flow; std::optional<Event> with a
    // value when an assign-time bytes-cap check fails and raises an
    // internal event. The shared
    // run_procedure() loop re-processes the source state with that event
    // so a fixture's `<transition event="error.execution">` can pick it up.

    [[nodiscard]] std::optional<Event> executeTransitionActions([[maybe_unused]] State source, [[maybe_unused]] std::size_t trIndex) {
        if (source == State::RequestSeed) {
            if (trIndex == 0) {
                {
                    auto _scope_tmp = std::vector<uint8_t>(pendingEventData_.begin(), pendingEventData_.end());
                    if (_scope_tmp.size() > 64) {
                        return Event::ErrorExecution;
                    }
                    seed_ = std::move(_scope_tmp);
                }
            }
        }
        if (source == State::Retry) {
            if (trIndex == 0) {
                retryCount_ = retryCount_ + 1;
            }
        }
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
    uint32_t ecuAddr_{};
    std::vector<uint8_t> seed_;
    int32_t maxRetries_ = 3;
    int32_t retryCount_ = 0;

    // ── Service handler (for <send sce:service>) ─────────────────
    SCE::Forge::ProcedureServiceHandler serviceHandler_;

    // ── Helper DI closures (for <sce:helper> declarations) ───────
    // Default-initialised to fail-fast throwing lambdas; the user must call
    // the matching set*() setter before runToCompletion().
    std::function<std::vector<uint8_t>(const std::vector<uint8_t>&)> computeKey_ = [](const std::vector<uint8_t>&) -> std::vector<uint8_t> { throw std::runtime_error("helper 'computeKey' not set — call setComputeKey() before runToCompletion()"); };

    // ── Done data storage ────────────────────────────────────────
    std::map<std::string, std::string> doneData_;

    // ── W3C SCXML 5.10: _event.data binding ──────────────────────
    std::string pendingEventData_;
};

// ── Convenience wrapper function ─────────────────────────────────

inline SCE::Forge::ProcedureRunResult execute(
    SCE::Forge::ProcedureServiceHandler handler,
    std::function<std::vector<uint8_t>(const std::vector<uint8_t>&)> computeKey,
    uint32_t ecuAddr) {
    ProcedureSecurityAccess sm;
    sm.setServiceHandler(std::move(handler));
    sm.setComputeKey(std::move(computeKey));
    sm.setEcuAddr(ecuAddr);
    return sm.runToCompletion();
}

}  // namespace SCE::Generated::ProcedureSecurityAccess

#endif  // SCE_FORGE_PROCEDURE_SECURITY_ACCESS_L2_H
