// SCE-GENERATED — DO NOT EDIT
// source-hash: 81cc59c6b8f46736a80b4b69ee489274d9b0ae32efae821805ba44c3bac3af2c
// SCE-MAP: record_closed.scxml:26 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: record_list.closed
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum RecordClosed {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "record_list.closed"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 143

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:record_closed:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:record_closed:1\",\"x-sce-source-hash\":\"81cc59c6b8f46736a80b4b69ee489274d9b0ae32efae821805ba44c3bac3af2c\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"command\":{\"type\":\"string\",\"pattern\":\"^(0|[1-9][0-9]{0,18}|1[0-7][0-9]{18}|18[0-3][0-9]{17}|184[0-3][0-9]{16}|1844[0-5][0-9]{15}|18446[0-6][0-9]{14}|184467[0-3][0-9]{13}|1844674[0-3][0-9]{12}|184467440[0-6][0-9]{10}|1844674407[0-2][0-9]{9}|18446744073[0-6][0-9]{8}|1844674407370[0-8][0-9]{6}|18446744073709[0-4][0-9]{5}|184467440737095[0-4][0-9]{4}|18446744073709550[0-9]{3}|18446744073709551[0-5][0-9]{2}|1844674407370955160[0-9]|1844674407370955161[0-5])$\"},\"operations\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"reason\":{\"anyOf\":[{\"enum\":[\"notWriter\",\"noRoomForValue\",\"noRoomForHistory\",\"noRoomForDigest\",\"changeTooLarge\",\"registerNamedTwice\",\"clockExhausted\",\"peerRefused\"]},{\"type\":\"null\"}]},\"wallTime\":{\"type\":\"string\",\"pattern\":\"^(-([1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-8])|0|[1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-7])$\"},\"counter\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}},\"required\":[\"command\",\"operations\",\"reason\",\"wallTime\",\"counter\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct RecordClosedPayload: Equatable, Sendable {
    public var command: UInt64
    public var operations: UInt32
    public var reason: RecordRefusal?
    public var wallTime: Int64
    public var counter: UInt32

    public init(command: UInt64, operations: UInt32, reason: RecordRefusal?, wallTime: Int64, counter: UInt32) {
        self.command = command
        self.operations = operations
        self.reason = reason
        self.wallTime = wallTime
        self.counter = counter
    }
}
