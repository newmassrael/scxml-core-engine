// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceRuntime

// What a driver records of a machine's Mesh peers (SCE_MESH.md): the Swift twin of the Kotlin
// driver's `RecordedSends` and `ChildRouters`. The peer is not there: what a scenario holds a
// step to is that the machine asked for the send.

/// What a machine sent its Mesh peers (a `<send>` whose target is `#name`) as the router a host
/// registers is handed it. Register `router` with the machine's `registerMeshRouter` before it
/// starts, so that what its start sends is recorded too; with no router registered such a send
/// is `error.execution` and reaches nobody. Its `take` is what a driver hands `replay` as `sent`.
final class RecordedSends {
    private var sends: [JSONValue] = []

    /// The router to register: it writes the send down and answers nothing.
    var router: HostSendHandler {
        { [self] request in
            record(request, invoke: nil, start: 0)
            return []
        }
    }

    /// Writes `request` down as a scenario's `sent` writes it: the target as the document
    /// writes it, the event's name, and the event's data as the JSON value its text spells,
    /// left out when the send carried none. A child's send (`ChildRouters`) carries the path of
    /// invoke ids that started it (`invoke`) and the `start` of that path; the machine's own
    /// carries neither.
    func record(_ request: HostSendRequest, invoke: String?, start: Int) {
        var members: [(key: String, value: JSONValue)] = [
            (key: "target", value: .text(request.target)),
            (key: "event", value: .text(request.eventName)),
        ]
        if !request.eventData.isEmpty, let data = try? parseJSON(request.eventData) {
            members.append((key: "data", value: data))
        }
        if let invoke {
            members.append((key: "invoke", value: .text(invoke)))
            members.append((key: "start", value: .number(String(start))))
        }
        sends.append(.object(members))
    }

    /// The sends made since this was last called, oldest first, the machine's and its
    /// children's.
    func take() -> [JSONValue] {
        defer { sends = [] }
        return sends
    }
}

/// The routers a driver gives the child sessions of the machine it drives (SCE_MESH.md, Child
/// Sessions in Mesh). Register `factory` with the machine's `registerChildMeshRouters` before
/// it starts: it answers a router for every child, which writes what the child sends into
/// `sends` under the path of invoke ids that started it and the start of that path, holds what
/// a step's `invoke` hands the child (`deliver`) until the parent applies it, and answers for
/// the child's own children under `<path>/<invoke>`.
final class ChildRouters {
    private let sends: RecordedSends
    private var starts: [String: Int] = [:]
    private var running: [String: ScenarioChildRouter] = [:]

    init(sends: RecordedSends) {
        self.sends = sends
    }

    /// The factory to register.
    var factory: ChildMeshRouters {
        { [self] invokeID, _ in open(invokeID) }
    }

    /// A router for a new start of `path`, which is now the session the path is running.
    fileprivate func open(_ path: String) -> ScenarioChildRouter {
        let number = (starts[path] ?? 0) + 1
        starts[path] = number
        let router = ScenarioChildRouter(owner: self, path: path, start: number)
        running[path] = router
        return router
    }

    /// Hands `event`, with `data` as its `_event.data`, to the running child of `path`, as a
    /// step hands one to the machine; false when no child of that path is running.
    func deliver(path: String, event: String, data: String) -> Bool {
        guard let router = running[path] else {
            return false
        }
        router.inbox.append(.raise(name: event, metadata: EventMetadata(data: data, eventType: .externalEvent)))
        return true
    }

    fileprivate func record(_ request: HostSendRequest, path: String, start: Int) {
        sends.record(request, invoke: path, start: start)
    }

    /// The path is no longer running this session: a step that names it fails until it starts
    /// again.
    fileprivate func detached(_ router: ScenarioChildRouter) {
        if running[router.path] === router {
            running[router.path] = nil
        }
    }
}

/// The router of one start of a child path.
fileprivate final class ScenarioChildRouter: ChildMeshRouter {
    private unowned let owner: ChildRouters
    let path: String
    let start: Int
    var inbox: [MeshCall] = []

    init(owner: ChildRouters, path: String, start: Int) {
        self.owner = owner
        self.path = path
        self.start = start
    }

    func send(_ request: HostSendRequest) -> [HostSendResponse] {
        owner.record(request, path: path, start: start)
        return []
    }

    /// A request the child makes is started and never answered: no peer is there.
    func invoke(_ event: HostInvokeEvent) -> HostInvokeResponse? {
        nil
    }

    func takeCalls() -> [MeshCall] {
        defer { inbox = [] }
        return inbox
    }

    func detach() {
        owner.detached(self)
    }

    func childRouter(invokeID: String, document: String) -> (any ChildMeshRouter)? {
        owner.open("\(path)/\(invokeID)")
    }
}

/// The recorders of one driven machine's Mesh peers, registered together.
final class PeerRecorder {
    let sends = RecordedSends()
    let children: ChildRouters

    init() {
        children = ChildRouters(sends: sends)
    }
}
