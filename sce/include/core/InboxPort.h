// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The inbox port of a C++ machine: what a run tells a host of each event the
// engine puts on its external queue, so the run can be written as an inbox
// history and judged linearizable to the promise of the inbox
// (docs/adr/0147, D3; docs/formal/InboxService.tla).
//
// The engine tells the port twice for each event it puts on its external queue
// (`StaticExecutionEngine::raiseExternal`, the one door every external enqueue of
// the machine goes through): when it begins to put the event there, ahead of the
// drain that applies what its inbox holds first (docs/adr/0084, D2 as amended at
// step 9), and immediately after the event entered. That drain's read of the
// inbox is where the event linearizes: what the inbox held then enters ahead of
// it, and a post made after it may follow it. The inputs the drain applies are
// told inside the event's two calls, each with its own two. Each call says
// whether the inbox held the event, an input a poster posted that the engine
// applied, or the machine raised it itself. A host reads one clock in both calls
// and has each `enqueue` of the history `sce-codegen check-inbox-history` judges
// (tests/forge/conformance/inbox_history.schema.json).
//
// A machine tells the port only when its build asks for it: its policy's
// `INBOX_PORT` (`InboxPortTrait`), which a generated machine takes from its
// build, `<NAME>_INBOX_PORT`, whose default is `SCE_INBOX_PORT` below, 0 unless
// set. Off, the engine holds no port (`InboxPortSlot` is an empty base) and each
// call is a discarded `if constexpr` branch, so the machine is what it was before
// the port existed. On, a host sets the port with `setInboxPort`, and with no
// port set each external enqueue asks one `std::function`. The port reads and is
// never asked anything, so a machine that tells its enqueues takes the steps one
// that does not takes.
//
// The cross-language siblings are the C11 machine's
// `<PREFIX><NAME>_external_enqueue` (`SCE_INBOX_PORT`, sce/types.h), the Go
// engine's `SetInboxPort`, and the Rust runtime's external queue a policy
// supplies (docs/adr/0076). Header-only.

#pragma once

#include <functional>
#include <string_view>
#include <type_traits>

// docs/adr/0147, D3: whether the machines of this build tell an inbox port each
// event they put on their external queue. Off by default. A generated machine's
// own `<NAME>_INBOX_PORT` takes this value unless the build sets that one, so a
// build can tell the port of one machine of several.
#ifndef SCE_INBOX_PORT
#define SCE_INBOX_PORT 0
#endif

namespace SCE::Core {

/// One of the two calls the inbox port is told for an event the engine puts on
/// its external queue. What it names is valid for the length of the call.
struct InboxEnqueue {
    /// The event, by the name `_event.name` holds when it is taken: the longer
    /// name a door resolved to the document's event, or else that event's own.
    std::string_view event;
    /// Its `_event.data`, as the queue holds it.
    std::string_view data;
    /// True for an input the engine applied from its inbox; false for an event
    /// of the machine's own: a `<send>` to itself, a delayed send come due, an
    /// ending or a report a host gave it on the owner, the host's own raise.
    bool posted = false;
    /// False when the engine begins to put the event on its queue, ahead of the
    /// drain before it; true once the event has entered.
    bool entered = false;
};

/// What `setInboxPort` takes: told each call on the thread that drives the
/// engine, inside the call that drove it. It must not call back into the engine.
using InboxPort = std::function<void(const InboxEnqueue &)>;

/// Whether the machines of policy `P` tell an inbox port: `P::INBOX_PORT`, and
/// false for a policy that declares none, as every policy written by hand.
template <typename P, typename = void> struct InboxPortTrait : std::false_type {};

template <typename P>
struct InboxPortTrait<P, std::void_t<decltype(P::INBOX_PORT)>> : std::bool_constant<static_cast<bool>(P::INBOX_PORT)> {
};

/// Where an engine holds its port: an empty base when its machine tells none,
/// which costs the engine no byte, and the port when it does.
template <bool Told> struct InboxPortSlot {};

template <> struct InboxPortSlot<true> {
    InboxPort inboxPort_;
};

}  // namespace SCE::Core
