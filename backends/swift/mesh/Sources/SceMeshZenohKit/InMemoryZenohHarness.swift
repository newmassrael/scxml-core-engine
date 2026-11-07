// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceMesh

/// The in-memory router as a harness of the conformance scenarios: it connects clients on a router of its own,
/// and its `endSessions` ends every session its clients hold as the router going away does and refuses new
/// ones until `restore`. It is how the kit holds the in-memory router itself to what a real one is held to.
public final class InMemoryZenohHarness: ZenohDisruptable {
    public let router = InMemoryZenohRouter()
    private let factory: InMemoryZenohFactory

    public init() {
        factory = InMemoryZenohFactory(router: router)
    }

    public func connect() throws(ZenohClientError) -> any ZenohClient {
        try factory.connect()
    }

    public func endSessions() {
        factory.setDown(true)
        for client in factory.clients {
            router.drop(client, words: "the router went away")
        }
    }

    public func restore() {
        factory.setDown(false)
    }
}
