// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceRuntime

// `_event.type` is decided where the engine dequeues an event, by the name for the events the
// processor raises and by the queue for the rest. The Rust runtime holds the same table, and the
// W3C lane of the tests package drives the engine through it (W3C 331).

@Test(arguments: [
    ("foo", false, EventType.internalEvent),
    ("foo", true, EventType.externalEvent),
    ("error.execution", false, EventType.platformEvent),
    ("error.communication", true, EventType.platformEvent),
    ("done.state.s1", false, EventType.platformEvent),
    ("done.invoke.child", true, EventType.platformEvent),
    ("errors", false, EventType.internalEvent),
    ("donee", true, EventType.externalEvent),
])
func anEventIsTypedByItsNameAndItsQueue(name: String, external: Bool, expected: EventType) {
    #expect(EventType.classify(eventName: name, fromExternalQueue: external) == expected)
}
