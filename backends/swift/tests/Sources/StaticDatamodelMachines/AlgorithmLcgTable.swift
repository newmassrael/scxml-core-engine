// SCE-GENERATED — DO NOT EDIT
// source-hash: f6c860c2693fcb3366bc871ff801c9b0ee9e491a1759830f8d83238267cc3b45
// SCE-MAP: algorithm_lcg_table.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmLcgTable`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmLcgTable.algorithmLcgTable(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum AlgorithmLcgTable {
    public static let LCG_TABLE: [UInt32] = [12345, 87628868, 71072467, 2332836374, 2726892157, 3908547000, 483019191, 2129828778, 2355140353, 2560230508, 3364893915, 171172990, 3194601925, 4148119648, 316399679, 3004788882]


    public static func algorithmLcgTable(_ n: UInt8) -> UInt32 {
        return LCG_TABLE[Int(n & 15)]
    }
}
