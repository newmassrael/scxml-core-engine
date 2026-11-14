// SCE-GENERATED — DO NOT EDIT
// source-hash: 7cb5c601bc5bd6648425cb4d614d7be9f82d1e41816b0f69f2fcd5bc5c43d40b
// SCE-MAP: local_sub_table.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="bounded-collection")
// Runtime: self-contained — Swift stdlib only
// Do not edit — regenerate from the source SCXML file.
//
// SCE Protocol-Synthesis RFC (section synth-5-L) bounded-collection: a typed container of a fixed capacity
// whose occupancy varies at run time. The storage is `[Element?]` slots, an occupancy array and a
// generation counter per slot; a handle packs the slot and its generation into a `UInt32` the same
// 16/16 way every other backend does, so a stale handle (one whose slot was removed and reused) is
// refused and not followed. The collection is a value: a copy holds its own slots.
//
// Operations contract: insert / remove / get / getBySlot / findByIndex (if index-by is set) /
// iteration (`Sequence`) / len / capacity.

/// Slot-index bit width inside the packed `LocalSubTableHandle.raw`: 65536 slots by 65536 generations,
/// the layout of the C11 `uint32_t` and the C++ `std::uint32_t` handle.
private let LocalSubTableSlotBits: UInt32 = 16
private let LocalSubTableSlotMask: UInt32 = 0xFFFF
private let LocalSubTableGenMask: UInt32 = 0xFFFF

/// A stable handle into `LocalSubTable`. Each `remove` then `insert` that reuses a slot increments the
/// slot's generation, so a handle taken before the removal finds nothing.
public struct LocalSubTableHandle: Hashable, Sendable {
    public let raw: UInt32

    public init(raw: UInt32) {
        self.raw = raw
    }

    /// The slot index (the low 16 bits).
    public var slot: UInt32 { raw & LocalSubTableSlotMask }

    /// The generation counter (the high 16 bits).
    public var generation: UInt32 { (raw >> LocalSubTableSlotBits) & LocalSubTableGenMask }

    /// Packs a `(slot, generation)` pair into the `UInt32` a handle holds.
    public static func pack(slot: UInt32, generation: UInt32) -> LocalSubTableHandle {
        LocalSubTableHandle(raw: ((generation & LocalSubTableGenMask) << LocalSubTableSlotBits) | (slot & LocalSubTableSlotMask))
    }
}

/// The result of `LocalSubTable.insert`. Under `<sce:on-overflow>oldest-wins</sce:on-overflow>` it is
/// always `ok` (the oldest entry is evicted to make room); under `reject` and `diagnostic-event` it is
/// `full` when every slot is occupied.
public enum LocalSubTableInsertResult: Equatable, Sendable {
    case ok(LocalSubTableHandle)
    case full
}

/// `LocalSubTable`: a bounded collection over `SubscriptionEntry`. Capacity `8`,
/// `<sce:on-overflow>diagnostic-event</sce:on-overflow>`, `<sce:ordering>insertion</sce:ordering>`,
/// `<sce:concurrency>single-writer</sce:concurrency>`.
public struct LocalSubTable: Sequence, Sendable {
    /// The slot capacity (`<sce:capacity>`).
    public static let capacity: Int = 8

    private var slots: [SubscriptionEntry?] = Array(repeating: nil, count: LocalSubTable.capacity)
    private var occupied: [Bool] = Array(repeating: false, count: LocalSubTable.capacity)
    private var generations: [UInt32] = Array(repeating: 0, count: LocalSubTable.capacity)
    private var count: Int = 0

    public init() {}

    /// The number of occupied slots.
    public var len: Int { count }

    /// Whether no slot is occupied.
    public var isEmpty: Bool { count == 0 }

    /// The slot capacity.
    public var capacity: Int { Self.capacity }

    /// Inserts `element`. A free slot is taken when there is one. When every slot is occupied the
    /// insert is `full`.
    public mutating func insert(_ element: SubscriptionEntry) -> LocalSubTableInsertResult {
        if let free = occupied.firstIndex(of: false) {
            slots[free] = element
            occupied[free] = true
            count += 1
            return .ok(.pack(slot: UInt32(free), generation: generations[free]))
        }
        return .full
    }

    /// Removes the entry `handle` names. `false` when the slot is free, or when the handle's
    /// generation is not the slot's current one (a use after remove).
    @discardableResult
    public mutating func remove(_ handle: LocalSubTableHandle) -> Bool {
        let index = Int(handle.slot)
        guard index < Self.capacity, occupied[index], generations[index] == handle.generation else {
            return false
        }
        slots[index] = nil
        occupied[index] = false
        generations[index] = (generations[index] &+ 1) & LocalSubTableGenMask
        count -= 1
        return true
    }

    /// The entry `handle` names, and `nil` when the slot is free or the handle's generation is not
    /// the slot's current one (a use after remove).
    public func get(_ handle: LocalSubTableHandle) -> SubscriptionEntry? {
        let index = Int(handle.slot)
        guard index < Self.capacity, occupied[index], generations[index] == handle.generation else {
            return nil
        }
        return slots[index]
    }

    /// The entry in `slot`, read without a handle: `nil` when `slot` is past the capacity or the slot
    /// is free. The generation is not checked, since the use-after-remove guard belongs to the handle;
    /// this is for walking the slots by index (the lowering of `<sce:foreach in="<collection>">`).
    public func getBySlot(_ slot: UInt32) -> SubscriptionEntry? {
        let index = Int(slot)
        guard index < Self.capacity, occupied[index] else { return nil }
        return slots[index]
    }

    /// The occupied entries in slot-index order. Under `<sce:ordering>insertion</sce:ordering>` that is
    /// the order of insertion, as the parse-time gates validate.
    public func makeIterator() -> AnyIterator<SubscriptionEntry> {
        var index = 0
        let snapshot = self
        return AnyIterator {
            while index < Self.capacity {
                defer { index += 1 }
                if snapshot.occupied[index], let element = snapshot.slots[index] {
                    return element
                }
            }
            return nil
        }
    }
}
