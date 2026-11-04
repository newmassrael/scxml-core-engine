// SCE-GENERATED — DO NOT EDIT
// source-hash: 4287da083ec5d997a88fc05438a17f332e7cee73dc1a7ecda505005ac5b92936
// SCE-MAP: bc_recent_table.scxml:5 :: _forge_body

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

/// Slot-index bit width inside the packed `BcRecentTableHandle.raw`: 65536 slots by 65536 generations,
/// the layout of the C11 `uint32_t` and the C++ `std::uint32_t` handle.
private let BcRecentTableSlotBits: UInt32 = 16
private let BcRecentTableSlotMask: UInt32 = 0xFFFF
private let BcRecentTableGenMask: UInt32 = 0xFFFF

/// A stable handle into `BcRecentTable`. Each `remove` then `insert` that reuses a slot increments the
/// slot's generation, so a handle taken before the removal finds nothing.
public struct BcRecentTableHandle: Hashable, Sendable {
    public let raw: UInt32

    public init(raw: UInt32) {
        self.raw = raw
    }

    /// The slot index (the low 16 bits).
    public var slot: UInt32 { raw & BcRecentTableSlotMask }

    /// The generation counter (the high 16 bits).
    public var generation: UInt32 { (raw >> BcRecentTableSlotBits) & BcRecentTableGenMask }

    /// Packs a `(slot, generation)` pair into the `UInt32` a handle holds.
    public static func pack(slot: UInt32, generation: UInt32) -> BcRecentTableHandle {
        BcRecentTableHandle(raw: ((generation & BcRecentTableGenMask) << BcRecentTableSlotBits) | (slot & BcRecentTableSlotMask))
    }
}

/// The result of `BcRecentTable.insert`. Under `<sce:on-overflow>oldest-wins</sce:on-overflow>` it is
/// always `ok` (the oldest entry is evicted to make room); under `reject` and `diagnostic-event` it is
/// `full` when every slot is occupied.
public enum BcRecentTableInsertResult: Equatable, Sendable {
    case ok(BcRecentTableHandle)
    case full
}

/// `BcRecentTable`: a bounded collection over `BcEntry`. Capacity `2`,
/// `<sce:on-overflow>oldest-wins</sce:on-overflow>`, `<sce:ordering>insertion</sce:ordering>`,
/// `<sce:concurrency>single-writer</sce:concurrency>`.
public struct BcRecentTable: Sequence, Sendable {
    /// The slot capacity (`<sce:capacity>`).
    public static let capacity: Int = 2

    private var slots: [BcEntry?] = Array(repeating: nil, count: BcRecentTable.capacity)
    private var occupied: [Bool] = Array(repeating: false, count: BcRecentTable.capacity)
    private var generations: [UInt32] = Array(repeating: 0, count: BcRecentTable.capacity)
    private var count: Int = 0

    public init() {}

    /// The number of occupied slots.
    public var len: Int { count }

    /// Whether no slot is occupied.
    public var isEmpty: Bool { count == 0 }

    /// The slot capacity.
    public var capacity: Int { Self.capacity }

    /// Inserts `element`. A free slot is taken when there is one. When every slot is occupied the
    /// insert is `full`, except under `oldest-wins`, which evicts the
    /// oldest-occupied slot and reuses it: the parse-time validator pins the ordering to insertion
    /// under that policy, so the lowest occupied slot index is the oldest entry.
    public mutating func insert(_ element: BcEntry) -> BcRecentTableInsertResult {
        if let free = occupied.firstIndex(of: false) {
            slots[free] = element
            occupied[free] = true
            count += 1
            return .ok(.pack(slot: UInt32(free), generation: generations[free]))
        }
        let evicted = occupied.firstIndex(of: true) ?? 0
        slots[evicted] = element
        generations[evicted] = (generations[evicted] &+ 1) & BcRecentTableGenMask
        // One evicted and one inserted: the count does not change.
        return .ok(.pack(slot: UInt32(evicted), generation: generations[evicted]))
    }

    /// Removes the entry `handle` names. `false` when the slot is free, or when the handle's
    /// generation is not the slot's current one (a use after remove).
    @discardableResult
    public mutating func remove(_ handle: BcRecentTableHandle) -> Bool {
        let index = Int(handle.slot)
        guard index < Self.capacity, occupied[index], generations[index] == handle.generation else {
            return false
        }
        slots[index] = nil
        occupied[index] = false
        generations[index] = (generations[index] &+ 1) & BcRecentTableGenMask
        count -= 1
        return true
    }

    /// The entry `handle` names, and `nil` when the slot is free or the handle's generation is not
    /// the slot's current one (a use after remove).
    public func get(_ handle: BcRecentTableHandle) -> BcEntry? {
        let index = Int(handle.slot)
        guard index < Self.capacity, occupied[index], generations[index] == handle.generation else {
            return nil
        }
        return slots[index]
    }

    /// The entry in `slot`, read without a handle: `nil` when `slot` is past the capacity or the slot
    /// is free. The generation is not checked, since the use-after-remove guard belongs to the handle;
    /// this is for walking the slots by index (the lowering of `<sce:foreach in="<collection>">`).
    public func getBySlot(_ slot: UInt32) -> BcEntry? {
        let index = Int(slot)
        guard index < Self.capacity, occupied[index] else { return nil }
        return slots[index]
    }

    /// The occupied entries in slot-index order. Under `<sce:ordering>insertion</sce:ordering>` that is
    /// the order of insertion, as the parse-time gates validate.
    public func makeIterator() -> AnyIterator<BcEntry> {
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
