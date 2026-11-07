// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceMesh
import Synchronization

/// A client factory for an `InMemoryZenohRouter`, whose router a test can make unreachable: while it is down
/// every `connect` fails, as a router that does not answer does. It remembers the clients it made, so a test
/// can end the session of the latest.
public final class InMemoryZenohFactory: ZenohClientFactory {
    private struct State {
        var down = false
        var attempts = 0
        var clients: [InMemoryZenohClient] = []
    }

    private let router: InMemoryZenohRouter
    private let state = Mutex(State())

    public init(router: InMemoryZenohRouter) {
        self.router = router
    }

    public func connect() throws(ZenohClientError) -> any ZenohClient {
        let refused = state.withLock { state -> Bool in
            state.attempts += 1
            return state.down
        }
        if refused {
            throw ZenohClientError("the router does not answer")
        }
        let client = router.connect()
        state.withLock { $0.clients.append(client) }
        return client
    }

    /// Whether the router refuses connections now.
    public func setDown(_ down: Bool) {
        state.withLock { $0.down = down }
    }

    /// How many times `connect` was called, the refused ones included.
    public var attempts: Int {
        state.withLock { $0.attempts }
    }

    /// The client of the most recent connection that was made, if any.
    public var latest: InMemoryZenohClient? {
        state.withLock { $0.clients.last }
    }
}
