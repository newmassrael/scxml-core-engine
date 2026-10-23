// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Synchronization

/// An event sent from a child to its parent (§scxml-6.4).
public struct ParentEvent: Sendable, Equatable {
    public var name: String
    public var data: String

    public init(name: String, data: String) {
        self.name = name
        self.data = data
    }
}

/// The events a child has sent its parent until the parent drains them (§scxml-6.4:
/// they are the parent's external events from the moment they are sent). It is
/// unbounded and keeps send order.
///
/// The Go runtime held these in a channel buffered at 100, written with a blocking
/// send, and a child that ran on the parent's goroutine blocked forever on its 101st
/// event in one tick. This is the shape the Rust runtime has (an `Arc<Mutex<VecDeque>>`):
/// the only bound on it is memory, as for every other queue here.
///
/// The one place the engine shares state with another thread: a child's host may
/// push from its own, which is why the events sit behind a mutex.
public final class ParentEventQueue: Sendable {
    private let events = Mutex<[ParentEvent]>([])

    public init() {}

    /// Appends one event. It never blocks for room.
    public func push(_ event: ParentEvent) {
        events.withLock { $0.append(event) }
    }

    /// How many events wait in the queue.
    public var count: Int {
        events.withLock { $0.count }
    }

    /// Removes and returns every queued event, oldest first.
    public func drain() -> [ParentEvent] {
        events.withLock { queued in
            let out = queued
            queued = []
            return out
        }
    }
}

/// Which reading of §scxml-B-2-8-1 a payload actually got.
///
/// The clause gives `_event.data` three readings and no fourth: content the
/// processor can interpret as XML becomes a DOM, content it can interpret as a value
/// becomes that value, and "otherwise, the Processor MUST treat the content as a
/// space-normalized string literal". Every engine here walks that ladder, and until
/// now every engine dropped which rung it landed on.
///
/// Dropping it is what makes a lost payload silent. Measured 2026-08-22 on three
/// independent Lua implementations, a host that hands over `{["milestone"]="refined"}`
/// (Lua's own table syntax) gets the third rung, and a document that then reads
/// `_event.data.milestone` assigns nothing. Nothing failed; the information stopped
/// existing.
///
/// `undecodable` is the one a host acts on, and it is not the engine guessing from a
/// leading brace: the script engine reports it because it attempted a structured read
/// and that read failed, which is a fact only the ladder holds.
///
/// Cross-language siblings: `SCE::PayloadReading` (C++) and
/// `sce_rust_runtime::PayloadReading`.
public enum PayloadReading: Sendable, Equatable {
    /// The event carried no payload, so no rung applies.
    case absent

    /// Rung one: read as an XML document, bound as a DOM.
    case dom

    /// Rung two: read as a value, bound as that value.
    case structured

    /// Rung three, and nothing suggested the content was structured. A `<content>`
    /// element holding prose lands here, and that is correct: W3C test 562 pins it.
    case text

    /// Rung three, taken after a structured read was attempted and failed. The
    /// payload announced itself as structure and the datamodel could not read it, so
    /// `_event.data` holds the raw characters and every `_event.data.<field>` the
    /// document reads is empty.
    case undecodable
}

/// Which third-rung reading a payload that fell through to text deserves.
///
/// The clause treats prose and a malformed object identically (both are "otherwise")
/// and a host does not. This is the one place that rule is written, so the ladder's
/// implementations mirror a definition and do not each decide for themselves what
/// "looks structured" means.
///
/// The test is the opening character, and deliberately only `{` and `[`. A number, a
/// bare word or a quoted string is what an author writes in a `<content>` element,
/// and W3C test 562 requires those to arrive as text without complaint; an object or
/// an array is what a host constructs, and nobody constructs one by accident.
/// Widening this to "anything not obviously prose" would report the ladder working as
/// a defect, which is the failure that gets a diagnostic ignored.
public func payloadReadingOfText(_ payload: String) -> PayloadReading {
    let trimmed = payload.utf8.drop { $0 == 0x20 || (0x09...0x0D).contains($0) }
    if trimmed.first == UInt8(ascii: "{") || trimmed.first == UInt8(ascii: "[") {
        return .undecodable
    }
    return .text
}
