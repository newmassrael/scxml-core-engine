// SCE-GENERATED — DO NOT EDIT
// source-hash: 047dc621dfd01b6348575becd26162c58fa20fa7680f076526766f91e97d2c5f
// SCE-MAP: sheet_cell.scxml:21 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: sheet.cell
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum SheetCell {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "sheet.cell"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct SheetCellPayload: Equatable, Sendable, SceDefault {
    public var row: UInt64 = 0
    public var col: UInt64 = 0
    public var value: Int64 = 0
    public var wallTime: Int64 = 0
    public var counter: UInt32 = 0
    public var nodeId: UInt64 = 0
    public var op: UInt8 = 0
    public var aRef: Bool = false
    public var aRow: UInt64 = 0
    public var aCol: UInt64 = 0
    public var aVal: Int64 = 0
    public var bRef: Bool = false
    public var bRow: UInt64 = 0
    public var bCol: UInt64 = 0
    public var bVal: Int64 = 0

    public init() {}

    public init(row: UInt64, col: UInt64, value: Int64, wallTime: Int64, counter: UInt32, nodeId: UInt64, op: UInt8, aRef: Bool, aRow: UInt64, aCol: UInt64, aVal: Int64, bRef: Bool, bRow: UInt64, bCol: UInt64, bVal: Int64) {
        self.row = row
        self.col = col
        self.value = value
        self.wallTime = wallTime
        self.counter = counter
        self.nodeId = nodeId
        self.op = op
        self.aRef = aRef
        self.aRow = aRow
        self.aCol = aCol
        self.aVal = aVal
        self.bRef = bRef
        self.bRow = bRow
        self.bCol = bCol
        self.bVal = bVal
    }
}
