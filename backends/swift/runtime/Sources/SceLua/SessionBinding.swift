// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import CLua

// The system variables and the one function a document reads about the session it runs in
// (§scxml-5.10): `_sessionid`, `_name`, `_ioprocessors` and `In()`. The values reach Lua as values,
// never as source text, so a session name with a quote in it is still one string.

/// What identifies a session to its own document.
public struct LuaSession: Equatable, Sendable {
    /// `_sessionid`: the system-generated id of this session, an NMTOKEN. It names the session and
    /// not the document, so two instances of one document do not read the same id.
    public var sessionID: String
    /// `_name`: the `name` attribute of the `<scxml>` element, or the identity the processor made up.
    public var name: String
    /// The address that reaches this session through the SCXML event processor. It is also what
    /// this session's own `<send>`s publish as `_event.origin`, so it is a global of its own.
    public var scxmlLocation: String
    /// The address of the BasicHTTP processor, once the deployment has declared an inbound endpoint.
    /// Support for that processor is optional and per deployment, so a session with no listener
    /// publishes no entry rather than an address nothing answers on.
    public var basicHTTPLocation: String?

    public init(sessionID: String, name: String, scxmlLocation: String, basicHTTPLocation: String? = nil) {
        self.sessionID = sessionID
        self.name = name
        self.scxmlLocation = scxmlLocation
        self.basicHTTPLocation = basicHTTPLocation
    }
}

/// What `In(state_id)` asks the host, which holds the active configuration.
private final class StateQueryBox {
    let query: (String) -> Bool

    init(_ query: @escaping (String) -> Bool) {
        self.query = query
    }
}

/// The C function the shim calls for `In`. It recovers the box from the context pointer and answers.
/// It does not raise and does not call back into the state.
private func askTheHost(_ context: UnsafeMutableRawPointer?, _ stateID: UnsafePointer<CChar>?, _ length: Int) -> Int32 {
    guard let context, let stateID else { return 0 }
    let box = Unmanaged<StateQueryBox>.fromOpaque(context).takeUnretainedValue()
    let name = String(decoding: UnsafeRawBufferPointer(start: stateID, count: length), as: UTF8.self)
    return box.query(name) ? 1 : 0
}

extension LuaState {
    /// Binds `_sessionid`, `_name`, `_scxml_location` and `_ioprocessors` (§scxml-C-1). Entry names
    /// and locations are what the other backends publish, so a machine reads the same table
    /// whichever one runs it. The read-only rule for these names is the generator's: an `<assign>` to
    /// one is lowered to `error.execution` and never reaches this state.
    public func bindSession(_ session: LuaSession) throws {
        try setGlobal("_sessionid", .string(session.sessionID))
        try setGlobal("_name", .string(session.name))
        try setGlobal("_scxml_location", .string(session.scxmlLocation))
        if let http = session.basicHTTPLocation, !http.isEmpty {
            try setGlobal("_sce_http_location", .string(http))
        } else {
            try setGlobal("_sce_http_location", .nilValue)
        }
        // A fixed chunk with nothing interpolated. Every entry is a table of its own: a document
        // that writes through one name must not change what the other answers.
        try execute(
            """
            do
                local location = _scxml_location
                _ioprocessors = {
                    ['http://www.w3.org/TR/scxml/#SCXMLEventProcessor'] = {location = location},
                    scxml = {location = location},
                }
                local http = _sce_http_location
                if http ~= nil then
                    _ioprocessors['http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor'] = {location = http}
                    _ioprocessors.basichttp = {location = http}
                end
                _sce_http_location = nil
            end
            """, name: "=bindSession")
    }

    /// Gives the session `In(state_id)`, which asks `query`; `nil` removes it. `query` runs inside a
    /// Lua call: it must not call into this state, and what it throws is not caught. An argument that
    /// is not a string raises in Lua before `query` is asked.
    public func setStateQuery(_ query: ((String) -> Bool)?) throws {
        guard let query else {
            stateQuery = nil
            try check(sce_lua_set_state_query(handle, nil, nil))
            return
        }
        let box = StateQueryBox(query)
        stateQuery = box
        try check(sce_lua_set_state_query(handle, askTheHost, Unmanaged.passUnretained(box).toOpaque()))
    }
}
