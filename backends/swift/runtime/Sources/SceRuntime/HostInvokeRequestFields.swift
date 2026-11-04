// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The fields of a typed request, read back out of the text the start site wrote them as
// (SCE Accepted Subset §2.12). A port of `backends/go/runtime/host_invoke_request.go`.
//
// A request that reaches a typed adapter was held to its record field by field where the
// invocation started, so none of these reads can fail on a value a host or a document
// can supply. One that does is a broken promise between two halves of generated code,
// and stops the process with a message that names the field, rather than hand the host a
// record the document never sent.

/// One schema of a machine's table of answer schemas (docs/adr/0087, section 4.1, item 2):
/// the schemas its invokes of `sce-host-model-generate` name with `sce:answer`, which a
/// generated machine declares as `<Machine>AnswerSchemas.all`. A host resolves the
/// `answer_schema` of a request through it, with no file and no second copy of a grammar,
/// and hands its provider the form it takes.
public struct AnswerSchema: Hashable, Sendable {
    /// `urn:sce:schema:<document name>:<revision>`: what the request carries.
    public let id: String
    /// The payload as a JSON Schema (2020-12), every bound as the build resolved it.
    public let jsonSchema: String
    /// The payload as a GBNF grammar, every text's bound counted in bytes.
    public let gbnf: String

    public init(id: String, jsonSchema: String, gbnf: String) {
        self.id = id
        self.jsonSchema = jsonSchema
        self.gbnf = gbnf
    }
}

extension HostInvokeRequest {
    /// The one text a typed request carries for `name`.
    public func requestText(_ name: String) -> String {
        guard let values = params[name], values.count == 1 else {
            preconditionFailure(
                "typed request \(invokeID.debugDescription) does not carry \(name.debugDescription) exactly once, though its record declares it"
            )
        }
        return values[0]
    }

    private func broken(_ name: String, _ text: String) -> Never {
        preconditionFailure(
            "typed request \(invokeID.debugDescription) carries \(name.debugDescription) as \(text.debugDescription), which its type does not parse, though the start site checked it"
        )
    }

    /// A whole-number field of a checked typed request, of the width the record declares.
    public func requestInteger<T: FixedWidthInteger>(_ name: String, as type: T.Type = T.self) -> T {
        let text = requestText(name)
        guard let value = T(text, radix: 10) else {
            broken(name, text)
        }
        return value
    }

    /// A fractional field of a checked typed request.
    public func requestReal<T: LosslessStringConvertible & BinaryFloatingPoint>(
        _ name: String,
        as type: T.Type = T.self
    ) -> T {
        let text = requestText(name)
        guard let value = T(text) else {
            broken(name, text)
        }
        return value
    }

    /// A truth-value field of a checked typed request: exactly `true` or `false`.
    public func requestBool(_ name: String) -> Bool {
        let text = requestText(name)
        switch text {
        case "true": return true
        case "false": return false
        default: broken(name, text)
        }
    }

    /// A text field of a checked typed request.
    public func requestString(_ name: String) -> String {
        requestText(name)
    }

    /// A byte-string field of a checked typed request, from the Latin-1 text the start
    /// site spelled it as: one scalar per byte.
    public func requestBytes(_ name: String) -> [UInt8] {
        let text = requestText(name)
        var out: [UInt8] = []
        out.reserveCapacity(text.unicodeScalars.count)
        for scalar in text.unicodeScalars {
            guard scalar.value <= 0xFF else {
                broken(name, text)
            }
            out.append(UInt8(scalar.value))
        }
        return out
    }
}
