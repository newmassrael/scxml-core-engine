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
 * @brief A reply from a host-served processor, turned back into an event.
 *
 * `std::nullopt` from a handler means "performed, no reply" — the common case
 * for a fire-and-forget act. A value is the request/reply shape: the engine
 * raises the named event on the EXTERNAL queue, which is where a reply from
 * outside the machine belongs (§scxml-C-1).
 *
 * A handler that cannot answer now — the usual case for real work — returns
 * `std::nullopt` and raises the reply later through its own handle to the
 * engine's queue. That is not a second mechanism: the engine does not
 * distinguish an event a handler raised from one it was handed.
 *
 * ONE reply per send, and that is the portable shape rather than a limit of
 * this engine. A C++ handler happens to be able to re-enter the engine and
 * enqueue more, because it is called through a `std::function` while only the
 * queue is being mutated. Its Rust sibling cannot: `handler_for` hands out a
 * `&mut` borrowed from the engine, so the engine is not reachable while the
 * handler runs. A host written against the C++ freedom therefore does not
 * port, and a document relying on it would be served by one engine and not the
 * other — the single-engine door this whole surface exists to remove.
 *
 * So an act whose outcome is two events belongs to the host's own loop: return
 * the first, deliver the second on the next step. Note the order if you do
 * re-enter on C++ anyway — what the handler raises is enqueued while it runs
 * and what it returns is enqueued after, so the RETURNED event arrives last,
 * which is the opposite of how the pair reads.
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
 * Returning a response is optional; see HostSendResponse for why the two
 * answers are not "delivered" and "failed". A handler that throws is a host
 * defect and is not caught here — the engine cannot invent a W3C-meaningful
 * outcome for it, and swallowing it would produce exactly the silence this
 * whole surface exists to remove.
 */
using HostSendHandler = std::function<std::optional<HostSendResponse>(const HostSendRequest &)>;

}  // namespace SCE
