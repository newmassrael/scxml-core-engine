// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The inbox port: what the owner of an engine does at the door of docs/adr/0084, told so a run
// of it can be written as an inbox history (docs/adr/0147, D3). The Swift sibling of the Kotlin
// runtime's `InboxPort` and of the C11 machine's `_external_enqueue`.

/// Told what the owner of an engine does at the door of docs/adr/0084 (`Engine.setInboxPort`):
/// each event it puts on the external queue, and the close of the inbox when a run ends, each as
/// two calls around it. A test reads one clock every thread of the run shares in both calls, and
/// writes the run as the inbox history `sce-codegen check-inbox-history` judges against
/// `docs/formal/InboxService.tla` (`tests/forge/conformance/inbox_history.schema.json`).
///
/// The calls are made on the thread that drives the engine, inside the call that drove it. The
/// port reads and is asked nothing, so an engine with one set does what one without does, and it
/// must not call back into the engine.
public protocol InboxPort<Event>: AnyObject {
    /// The event type of the machine whose owner tells this port.
    associatedtype Event

    /// The owner begins to put `event` on the external queue. `posted` is true for an input the
    /// owner applies from the inbox, and false for an event of its own: a `<send>` to its own
    /// session, a delayed send come due, a host-run invocation's ending made on the owner, the
    /// host's own raise. Putting an event of its own on the queue begins with the drain that
    /// applies what the inbox holds first (docs/adr/0084, D2 as amended at step 9), so this call
    /// comes before that drain, and the inputs it applies are told between this call and
    /// `enqueueEnds` for the same event.
    func enqueueBegins(_ event: EventWithMetadata<Event>, posted: Bool)

    /// `event`, told to `enqueueBegins` last with the same `posted`, has entered the external
    /// queue.
    func enqueueEnds(_ event: EventWithMetadata<Event>, posted: Bool)

    /// The inbox, open until now, begins to close: the run has ended or was stopped, and from
    /// `closeEnds` on every post is refused.
    func closeBegins()

    /// The inbox refuses posts from now on. What it held when it closed is applied after this
    /// call, each input told to `enqueueBegins` and `enqueueEnds`.
    func closeEnds()
}
