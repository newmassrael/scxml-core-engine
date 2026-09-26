// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include <cstddef>
#include <cstdint>
#include <functional>
#include <map>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

#include "core/InvokeHelper.h"

namespace SCE {

/**
 * @brief What a `<send>` addressed to a host-served processor said.
 *
 * §scxml-6.2.5 makes a `<send>` `type` an extensible identifier, so the set of
 * Event I/O Processors is open by design. SCE implements two of them; anything
 * else was refused with `error.execution` and no platform could widen the set —
 * a consumer could name a processor and be refused, but not name one and be
 * served. A host declares the types it serves at build time (so codegen emits a
 * dispatch instead of a refusal) and registers a handler for each at run time.
 *
 * The C++ port of `sce_rust_runtime::HostSendRequest`, field for field. The
 * Rust side landed first because the report came from a Rust consumer; keeping
 * the shape identical is what lets one host be described once and ported, and
 * it is the same reason `HttpSendRequest` here matches its Kotlin sibling.
 *
 * Every field is what the document wrote, not an interpretation of it: a
 * handler that wants to reject a malformed request needs to see the same thing
 * the author typed.
 */
struct HostSendRequest {
    /// The `type` this send named. Present even though the handler was looked
    /// up by it, because one handler may serve several types and would
    /// otherwise have to be told which it is by a capture per registration.
    std::string processorType;
    /// `<send event="...">`, or the value `eventexpr` evaluated to.
    std::string eventName;
    /// `<send target="...">`, empty when the document named none. §scxml-6.2
    /// leaves a target's meaning to the processor, so SCE passes it through
    /// without interpreting it.
    std::string target;
    /// Inline `<content>`, empty when the document carried none.
    std::string content;
    /// `<param>` values, keyed by name. A repeated name keeps every value in
    /// document order rather than the last one winning — §scxml-6.2 permits
    /// repetition and dropping it would lose data the author wrote.
    std::map<std::string, std::vector<std::string>> params;
    /// The send's id (§scxml-6.2.4), auto-generated when the document declared
    /// none. A handler correlating a reply, or honouring a `<cancel>`, needs it.
    std::string sendId;
};

/**
 * @brief One event a host-served act produced.
 *
 * The engine raises each on the EXTERNAL queue, which is where a reply from
 * outside the machine belongs (§scxml-C-1).
 *
 * A handler answers with a LIST of these, in the order the document should see
 * them — see HostSendHandler. Empty is "performed, nothing to report", which is
 * the common case for a fire-and-forget act and for real work that will answer
 * later through the host's own loop.
 */
struct HostSendResponse {
    /// Event to raise. A name the generated machine does not declare is
    /// dropped, matching what the engine does with any such event.
    std::string eventName;
    /// Payload for `_event.data`, empty for a bare reply.
    std::string eventData;
};

/**
 * @brief What a host registers for one declared processor type.
 *
 * Answers with the events the act produced, IN ORDER. A list rather than a
 * single reply, for one reason: an act can produce two observations that the
 * document must see in a particular order, and every other way of expressing
 * that costs portability or hides state.
 *
 * `examples/ai_loop/` is the case. Its `priming` state leaves on `prompt.sent`
 * — "the session has been told what it is here for" — and only then is the
 * machine somewhere a turn result means anything; its own comment says
 * reporting the turn first leaves the run sitting in `priming` forever. So
 * prompting a fresh session produces exactly two events with exactly one
 * correct order.
 *
 * The two alternatives were measured and rejected:
 *
 *   * Let the handler re-enter the engine and raise the extra events. A C++
 *     handler can (it is called through a `std::function` while only the queue
 *     is being mutated); its Rust sibling cannot, because `handler_for` hands
 *     out a `&mut` borrowed from the engine. A host written against the C++
 *     freedom would not port — the single-engine door this whole surface
 *     exists to remove. It also inverts the order, since what a handler raises
 *     is enqueued while it runs and what it returns is enqueued after.
 *   * Return one event and have the host deliver the rest on its next step.
 *     That works on both engines and puts a pending slot back in the host —
 *     the hidden host-side state that moving an act into the document is
 *     supposed to remove.
 *
 * A list needs no re-entrancy on any backend, so the engines are equivalent by
 * construction rather than by agreement, and the order is the one the host
 * wrote down.
 *
 * A handler that throws is a host defect and is not caught here — the engine
 * cannot invent a W3C-meaningful outcome for it, and swallowing it would
 * produce exactly the silence this whole surface exists to remove.
 */
using HostSendHandler = std::function<std::vector<HostSendResponse>(const HostSendRequest &)>;

/**
 * @brief An `<invoke>` the host runs, at the point the state was entered
 *
 * §scxml-6.4.1 leaves the invokable set to the platform in the same words
 * §scxml-6.2.5 uses for `<send>`, so a host may implement its own `type` here
 * too — but an invoke is not a send. It has a LIFETIME: it starts when the
 * state is entered, it is cancelled if the state exits, and the document may
 * be waiting on `done.invoke.<id>`. That is why the handler receives an event
 * rather than a bare request, and why this is a second registry rather than a
 * second use of the first: a host that can deliver an event is not thereby
 * able to run a process it must also be able to stop.
 *
 * The C++ port of `sce_rust_runtime::host_processor`'s invoker half, field for
 * field, for the reason the send half is a port: one host described once and
 * ported is the whole point of keeping the shapes identical.
 */
struct HostInvokeRequest {
    /// The `type` this `<invoke>` named.
    std::string processorType;
    /// The invoke's id (§scxml-6.4.1), auto-derived when the author declared
    /// none. This is the name the DOCUMENT waits on: a completion is
    /// `done.invoke.<invokeId>`, so a host finishing asynchronously must keep
    /// it.
    std::string invokeId;
    /// `<invoke src="...">`, empty when the document named none. SCE does not
    /// interpret it — what a src means is the invoked processor's business.
    std::string src;
    /// `<param>` values keyed by name; a repeated name keeps every value in
    /// document order.
    std::map<std::string, std::vector<std::string>> params;
    /// Inline `<content>`, empty when the document carried none.
    std::string content;
    /// Which start of this invoke this is. The engine assigns it, and a host
    /// that finishes later hands it back to `completeHostInvoke`.
    ///
    /// The id alone cannot say it: a state that exits and is entered again
    /// starts the same `<invoke>` a second time under the same id, and a result
    /// the first run produces after it was cancelled would otherwise read as
    /// the second run's (§scxml-6.4 — once the state has exited, what the
    /// cancelled process sends is ignored). Distinct for every start of every
    /// invocation within one engine.
    uint64_t token = 0;
};

/**
 * @brief An `<invoke>` the host was running, at the point its state exited
 */
struct HostInvokeCancel {
    /// The `type` the `<invoke>` named.
    std::string processorType;
    /// The invocation being cancelled — the same id its start carried.
    std::string invokeId;
    /// The token its start carried, so a host running more than one start of
    /// the same id stops the right one.
    uint64_t token = 0;
};

/**
 * @brief One turn of a host-run invoke's lifecycle
 *
 * Exactly one of `start` and `cancel` is engaged. Both arms go to ONE
 * registered handler rather than to two separately registered callbacks,
 * because a host that can start an invocation and cannot stop it is not a
 * working invoker — and two registrations make that state reachable. One
 * handler means the pair is registered together or not at all.
 */
struct HostInvokeEvent {
    /// §scxml-6.4: the state was entered and the macrostep has settled. Begin
    /// the invoked process.
    std::optional<HostInvokeRequest> start;
    /// §scxml-6.4: the state exited. Stop it.
    ///
    /// Delivered only for an invocation that is still running: one that never
    /// started (its state exited before the macrostep ended) has nothing to
    /// tear down, and one that already completed has nothing left to stop —
    /// `done.invoke` said the process is over.
    std::optional<HostInvokeCancel> cancel;
};

/**
 * @brief A host invoker's answer to a start
 *
 * Read only for a start; an answer to a cancel is ignored, because there is
 * nothing left for it to mean.
 */
struct HostInvokeResponse {
    /// Payload for an immediate `done.invoke.<invokeId>`, for an invocation
    /// that completed before returning.
    ///
    /// `std::nullopt` is the ordinary case: the work outlives the call, and
    /// the host reports it through `completeHostInvoke` with the request's
    /// token when it finishes. SCE does not synthesise a completion the host
    /// did not report — an invoked process that never terminates never fires
    /// `done.invoke`, which is what §scxml-6.4 says.
    std::optional<std::string> doneData;
};

/**
 * @brief A registered invoke-lifecycle handler
 */
using HostInvokeHandler = std::function<std::optional<HostInvokeResponse>(const HostInvokeEvent &)>;

/**
 * @brief The reserved `<param>` a host-run `<invoke>` names its deadline with,
 *        in milliseconds
 *
 * The engine reads it and does not hand it to the host: past the deadline, an
 * invocation still running is cancelled and the document receives
 * `error.invoke.<id>` instead of its completion. A neutral name rather than
 * `sce:mesh-rpc`'s `_mesh_deadline_ms`, so the two invoke types converge on one
 * spelling.
 */
inline constexpr std::string_view HOST_INVOKE_DEADLINE_PARAM = "_sce_deadline_ms";

/**
 * @brief Read the text of a HOST_INVOKE_DEADLINE_PARAM value as milliseconds
 *
 * One or more ASCII digits, optionally followed by `.` and one or more `0` — a
 * `<param expr>` reaches the request as the text of its value, and a script
 * engine may render a whole number `5000.0` — within a signed 64-bit count.
 * Anything else is `std::nullopt`: no sign, no whitespace, no exponent, no
 * digit separator.
 *
 * Spelled out rather than left to `strtoull`, which skips leading whitespace
 * and reads a sign, because every runtime implements this and the languages'
 * parsers disagree; the table they are all held to is
 * `sce-build/tests/fixtures/host_processor/host_invoke_deadline_values.json`.
 */
inline std::optional<uint64_t> parseHostInvokeDeadlineMs(std::string_view written) {
    const auto dot = written.find('.');
    const std::string_view whole = written.substr(0, dot);
    if (whole.empty()) {
        return std::nullopt;
    }
    constexpr uint64_t max = static_cast<uint64_t>(INT64_MAX);
    uint64_t ms = 0;
    for (const char c : whole) {
        if (c < '0' || c > '9') {
            return std::nullopt;
        }
        const auto digit = static_cast<uint64_t>(c - '0');
        if (ms > (max - digit) / 10) {
            return std::nullopt;
        }
        ms = ms * 10 + digit;
    }
    if (dot != std::string_view::npos) {
        const std::string_view fraction = written.substr(dot + 1);
        if (fraction.empty() || fraction.find_first_not_of('0') != std::string_view::npos) {
            return std::nullopt;
        }
    }
    return ms;
}

/**
 * @brief Which start of which host-run invocation a scheduled deadline ends
 */
struct HostInvokeDeadline {
    /// The invocation's `type`.
    std::string processorType;
    /// The invocation's id.
    std::string invokeId;
    /// The start this deadline belongs to; a restart under the same id has its
    /// own.
    uint64_t token = 0;
};

/**
 * @brief Whether an event named `eventName` carrying `_event.invokeid` =
 *        `invokeId` is the completion of a host-run invocation
 *
 * That is a `done.invoke.<id>` whose `<id>` is one of `hostInvokeIds`, or the
 * generic `done.invoke` a document that names no specific completion
 * receives, whose invokeid is one of them.
 *
 * Such an event is accepted only through `completeHostInvoke`, the one path
 * that knows the invocation is still running. Raised any other way it could
 * be a cancelled run's late reply (§scxml-6.4), so the engine refuses it.
 */
inline bool isHostInvokeCompletion(std::string_view eventName, std::string_view invokeId,
                                   const std::string_view *hostInvokeIds, std::size_t hostInvokeIdCount) {
    constexpr std::string_view prefix = ::SCE::Core::InvokeHelper::DONE_INVOKE_PREFIX;
    std::string_view id;
    if (eventName.substr(0, prefix.size()) == prefix) {
        id = eventName.substr(prefix.size());
    } else if (eventName == ::SCE::Core::InvokeHelper::DONE_INVOKE_EVENT) {
        // The generic `done.invoke` a document that names no specific
        // completion receives: its invokeid says whose completion it is.
        id = invokeId;
    } else {
        return false;
    }
    for (std::size_t i = 0; i < hostInvokeIdCount; ++i) {
        if (id == hostInvokeIds[i]) {
            return true;
        }
    }
    return false;
}

}  // namespace SCE
