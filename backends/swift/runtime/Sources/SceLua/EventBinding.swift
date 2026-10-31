// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// `_event`, the system variable a document reads the event being handled through (§scxml-5.10): its
// name and metadata, and the payload under `data`. The binding is rebuilt from nothing on every
// dequeue, so a field the new event does not carry is absent and never the previous event's.

/// The event a state machine is about to handle, as the engine hands it to the script engine.
public struct LuaEvent: Equatable, Sendable {
    /// The name the event arrived under; `_event.name`.
    public var name: String
    /// `platform`, `internal` or `external`; `_event.type`. The caller knows which queue the event
    /// came off and which names the processor raised itself, so it says; this layer does not guess.
    public var type: String?
    /// `_event.sendid`; absent for an event no `<send>` produced.
    public var sendID: String?
    /// `_event.origin` and `_event.origintype`: where a reply can be sent.
    public var origin: String?
    public var originType: String?
    /// `_event.invokeid`; absent for an event no invoked session produced.
    public var invokeID: String?
    /// The payload as the sender wrote it; `_event.raw`, and the text `_event.data` is read from.
    public var data: String?

    public init(
        name: String,
        type: String? = nil,
        sendID: String? = nil,
        origin: String? = nil,
        originType: String? = nil,
        invokeID: String? = nil,
        data: String? = nil
    ) {
        self.name = name
        self.type = type
        self.sendID = sendID
        self.origin = origin
        self.originType = originType
        self.invokeID = invokeID
        self.data = data
    }
}

/// Which reading of the payload `_event.data` took, in the terms of §scxml-B-2-8-1. The clause
/// mandates its last rung, the space-normalized string, and says nothing about telling anyone it was
/// reached, which is where a host's payload leaves the system quietly. `text` and `undecodable` are
/// that one rung, told apart by whether the payload announced structure and was refused.
public enum PayloadReading: Equatable, Sendable {
    /// No payload came with the event; `_event.data` is undefined.
    case absent
    /// The payload parsed as an XML document and became a DOM. Not produced yet: the DOM binding is
    /// not part of this module, so a document payload reads as `text`.
    case dom
    /// The payload parsed as structured data and became the corresponding value.
    case structured
    /// The payload is prose, and the clause requires it to arrive as a string without complaint.
    case text
    /// The payload announced structure (`{` or `[`) and the datamodel refused it. `_event.data`
    /// holds the same string as for `text`; what happened is entirely different.
    case undecodable

    /// Which of the two text readings applies to content that did not parse. Only `{` and `[`
    /// announce structure: a number, a bare word or a quoted string is what an author writes in a
    /// `<content>` element, and an object or an array is what a host constructs. The one rule the
    /// C11 runtime's `sce_payload_reading_of_text` and its siblings in the other backends apply.
    static func ofText(_ text: String) -> PayloadReading {
        let blanks: Set<UInt8> = [0x20, 0x09, 0x0A, 0x0D, 0x0C, 0x0B]
        guard let first = text.utf8.first(where: { !blanks.contains($0) }) else {
            return .absent
        }
        return first == UInt8(ascii: "{") || first == UInt8(ascii: "[") ? .undecodable : .text
    }
}

extension LuaState {
    /// The transient globals the binding chunk reads. They are removed again before the call
    /// returns, so a document never sees them.
    private static let bindingGlobals = [
        "_sce_event_name", "_sce_event_type", "_sce_event_sendid", "_sce_event_origin",
        "_sce_event_origintype", "_sce_event_invokeid", "_sce_event_raw",
    ]

    /// A fixed chunk with nothing interpolated: the payload is data, so it crosses as a Lua value and
    /// is decoded in Lua and never pasted into Lua source. The shape that did paste it ran the
    /// payload, so an event's data could execute code in the session, and a JSON object read as a
    /// Lua array. Answers whether the payload parsed as structured data.
    private static let bindingChunk = """
        (function()
            local raw = _sce_event_raw
            _event = {
                name = _sce_event_name, type = _sce_event_type, sendid = _sce_event_sendid,
                origin = _sce_event_origin, origintype = _sce_event_origintype,
                invokeid = _sce_event_invokeid, raw = raw,
            }
            local decoded = false
            if raw ~= nil and raw ~= '' then
                local value, ok = JSON._parse_document(raw)
                if ok then
                    _event.data = value
                    decoded = true
                else
                    _event.data = _scxml_normalize_ws(raw)
                end
            end
            _sce_event_name, _sce_event_type, _sce_event_sendid = nil, nil, nil
            _sce_event_origin, _sce_event_origintype, _sce_event_invokeid = nil, nil, nil
            _sce_event_raw = nil
            return decoded
        end)()
        """

    /// Binds `_event` for the event about to be handled, replacing the previous one whole, and says
    /// which reading the payload took. Needs the library of `loadEcmaScriptLibrary()`.
    @discardableResult
    public func bindCurrentEvent(_ event: LuaEvent) throws -> PayloadReading {
        func text(_ value: String?) -> LuaValue {
            // An empty string is no value: an event no `<send>` produced has no `sendid`, and Lua
            // reads that as nil, which ECMAScript surfaces as the same `undefined`.
            guard let value, !value.isEmpty else { return .nilValue }
            return .string(value)
        }
        do {
            try setGlobal("_sce_event_name", .string(event.name))
            try setGlobal("_sce_event_type", text(event.type))
            try setGlobal("_sce_event_sendid", text(event.sendID))
            try setGlobal("_sce_event_origin", text(event.origin))
            try setGlobal("_sce_event_origintype", text(event.originType))
            try setGlobal("_sce_event_invokeid", text(event.invokeID))
            try setGlobal("_sce_event_raw", text(event.data))
            let decoded = try evaluate(Self.bindingChunk, name: "=bindCurrentEvent")
            guard let payload = event.data, !payload.isEmpty else { return .absent }
            return decoded == .boolean(true) ? .structured : PayloadReading.ofText(payload)
        } catch {
            // A failure part way leaves the transient globals behind; take them away so the
            // next binding starts from nothing.
            for name in Self.bindingGlobals {
                try? setGlobal(name, .nilValue)
            }
            throw error
        }
    }
}
