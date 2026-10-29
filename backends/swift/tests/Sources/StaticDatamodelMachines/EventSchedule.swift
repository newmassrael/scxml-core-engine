// SCE-GENERATED — DO NOT EDIT
// source-hash: 1178ba59796ca1f7c46f8569fd4bfda85c2b97db65f51728f3615d990d97dd1c
// SCE-MAP: event_schedule.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: calendar.schedule
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum EventSchedule {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "calendar.schedule"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct EventSchedulePayload: Equatable, Sendable, SceDefault {
    public var zone: Int64 = 0
    public var start: Int64 = 0
    public var length: Int64 = 0
    public var freq: Int64 = 0
    public var interval: Int64 = 0
    public var days: Int64 = 0
    public var count: Int64 = 0
    public var until: Int64 = 0
    public var bymonth: Int64 = 0
    public var mdPos: Int64 = 0
    public var mdNeg: Int64 = 0
    public var invalid: Int64 = 0
    public var op0: Int64 = 0
    public var op1: Int64 = 0
    public var op2: Int64 = 0
    public var op3: Int64 = 0
    public var op4: Int64 = 0
    public var op5: Int64 = 0
    public var op6: Int64 = 0
    public var on0: Int64 = 0
    public var on1: Int64 = 0
    public var on2: Int64 = 0
    public var on3: Int64 = 0
    public var on4: Int64 = 0
    public var on5: Int64 = 0
    public var on6: Int64 = 0
    public var spPos: Int64 = 0
    public var spNeg: Int64 = 0
    public var wkst: Int64 = 0
    public var wnPos: Int64 = 0
    public var wnNeg: Int64 = 0
    public var yd0: Int64 = 0
    public var yd1: Int64 = 0
    public var yd2: Int64 = 0
    public var yd3: Int64 = 0
    public var yd4: Int64 = 0
    public var yd5: Int64 = 0
    public var ydn0: Int64 = 0
    public var ydn1: Int64 = 0
    public var ydn2: Int64 = 0
    public var ydn3: Int64 = 0
    public var ydn4: Int64 = 0
    public var ydn5: Int64 = 0
    public var gone: Int64 = 0
    public var flag: Int64 = 0

    public init() {}

    public init(zone: Int64, start: Int64, length: Int64, freq: Int64, interval: Int64, days: Int64, count: Int64, until: Int64, bymonth: Int64, mdPos: Int64, mdNeg: Int64, invalid: Int64, op0: Int64, op1: Int64, op2: Int64, op3: Int64, op4: Int64, op5: Int64, op6: Int64, on0: Int64, on1: Int64, on2: Int64, on3: Int64, on4: Int64, on5: Int64, on6: Int64, spPos: Int64, spNeg: Int64, wkst: Int64, wnPos: Int64, wnNeg: Int64, yd0: Int64, yd1: Int64, yd2: Int64, yd3: Int64, yd4: Int64, yd5: Int64, ydn0: Int64, ydn1: Int64, ydn2: Int64, ydn3: Int64, ydn4: Int64, ydn5: Int64, gone: Int64, flag: Int64) {
        self.zone = zone
        self.start = start
        self.length = length
        self.freq = freq
        self.interval = interval
        self.days = days
        self.count = count
        self.until = until
        self.bymonth = bymonth
        self.mdPos = mdPos
        self.mdNeg = mdNeg
        self.invalid = invalid
        self.op0 = op0
        self.op1 = op1
        self.op2 = op2
        self.op3 = op3
        self.op4 = op4
        self.op5 = op5
        self.op6 = op6
        self.on0 = on0
        self.on1 = on1
        self.on2 = on2
        self.on3 = on3
        self.on4 = on4
        self.on5 = on5
        self.on6 = on6
        self.spPos = spPos
        self.spNeg = spNeg
        self.wkst = wkst
        self.wnPos = wnPos
        self.wnNeg = wnNeg
        self.yd0 = yd0
        self.yd1 = yd1
        self.yd2 = yd2
        self.yd3 = yd3
        self.yd4 = yd4
        self.yd5 = yd5
        self.ydn0 = ydn0
        self.ydn1 = ydn1
        self.ydn2 = ydn2
        self.ydn3 = ydn3
        self.ydn4 = ydn4
        self.ydn5 = ydn5
        self.gone = gone
        self.flag = flag
    }
}
