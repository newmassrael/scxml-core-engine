// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include <functional>
#include <map>
#include <optional>
#include <string>
#include <vector>

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
};

/**
 * @brief An `<invoke>` the host was running, at the point its state exited
 */
struct HostInvokeCancel {
    /// The `type` the `<invoke>` named.
    std::string processorType;
    /// The invocation being cancelled — the same id its start carried.
    std::string invokeId;
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
    /// Delivered only for an invocation that actually started: a state that
    /// exits before the macrostep ends never runs its invoke, and cancelling
    /// something that never began would have the host tearing down state it
    /// never built.
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
    /// the host raises the completion itself when it finishes. SCE does not
    /// synthesise a completion the host did not report — an invoked process
    /// that never terminates never fires `done.invoke`, which is what
    /// §scxml-6.4 says.
    std::optional<std::string> doneData;
};

/**
 * @brief A registered invoke-lifecycle handler
 */
using HostInvokeHandler = std::function<std::optional<HostInvokeResponse>(const HostInvokeEvent &)>;

}  // namespace SCE
