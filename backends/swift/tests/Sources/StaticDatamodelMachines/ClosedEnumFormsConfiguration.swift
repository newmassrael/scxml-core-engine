// SCE-GENERATED — DO NOT EDIT
// source-hash: 75f53f6deefa1a9e19646e018ad72cacc30c4c7ca7f7f96f21be9f17f58cfdae
// SCE-MAP: closed_enum_forms_configuration.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceRuntime
import SceTestProfile

/// The test configuration `closed_enum_forms_configuration` (docs/SCE_TEST_PROFILE.md §11.6) as the
/// profile's runtime reads it, the members of a caseless enum of the module as every unit's are:
/// the configuration, the factory of its components' machines, the module parameters they read,
/// and the reader of the inputs a recorded trace carries on its system ports
/// (docs/SCE_TTCN3_LANE.md §10.5).
public enum ClosedEnumFormsConfiguration {
    /// The configuration: the MTC's component type and each PTC's, the ports and timers each type
    /// declares, each `<sce:connect>` and each `<sce:map>`.
    public static func configuration() -> SceTestProfile.Configuration {
        SceTestProfile.Configuration(
            mtcType: "closed_enum_forms_listener",
            ptcTypes: [:],
            portsOfType: [
                "closed_enum_forms_listener": ["sut"],
            ],
            timersOfType: [:],
            connections: [],
            mappings: [
                SceTestProfile.Mapping(SceTestProfile.Endpoint(component: "mtc", port: "sut"), systemPort: "sut"),
            ])
    }

    /// The machine of each component type of the configuration: the component the profile's
    /// runtime drives, generated beside its test component's machine (docs/SCE_TEST_PROFILE.md
    /// §12.3). Nil for a type the configuration does not name, which the run ends with `error`
    /// where it is created.
    public static func machine(_ componentType: String) -> (any SceTestProfile.ComponentMachine)? {
        switch componentType {
        case "closed_enum_forms_listener": return ClosedEnumFormsListenerComponent()
        default: return nil
        }
    }

    /// The module parameters the components of the configuration read (docs/SCE_TEST_PROFILE.md
    /// §11.5), each of which a suite that runs it gives a value.
    public static func moduleParameters() -> [String] {
        []
    }

    /// The record `order_placed` read from the canonical form of the test log
    /// (docs/SCE_TEST_PROFILE.md §12.5), as a recorded input trace holds it
    /// (docs/SCE_TTCN3_LANE.md §10.3). The inverse of `canonical_order_placed`.
    ///
    /// - Throws: `SceTestProfile.TraceRefusal` naming where its value does not belong to its type.
    public static func read_order_placed(_ value: SceRuntime.JSONValue) throws(SceTestProfile.TraceRefusal)
        -> OrderPlacedPayload
    {
        try SceTestProfile.FromCanonical.record(value, ["order_id", "lines", "note", "priority", "customer", "payment"], "order_placed") { (record_0: SceRuntime.JSONValue) throws(SceTestProfile.TraceRefusal) -> OrderPlacedPayload in try OrderPlacedPayload(order_id: SceTestProfile.FromCanonical.integer(SceTestProfile.FromCanonical.field(record_0, "order_id", "order_placed.order_id"), UInt32.self, "order_placed.order_id"), lines: SceTestProfile.FromCanonical.elements(SceTestProfile.FromCanonical.field(record_0, "lines", "order_placed.lines"), 0, 4, "order_placed.lines") { item_0 throws(SceTestProfile.TraceRefusal) in try SceTestProfile.FromCanonical.record(item_0, ["sku", "quantity", "color"], "order_placed.lines[]") { (record_1: SceRuntime.JSONValue) throws(SceTestProfile.TraceRefusal) -> OrderLinePayload in try OrderLinePayload(sku: SceTestProfile.FromCanonical.text(SceTestProfile.FromCanonical.field(record_1, "sku", "order_placed.lines[].sku"), 16, "order_placed.lines[].sku"), quantity: SceTestProfile.FromCanonical.integer(SceTestProfile.FromCanonical.field(record_1, "quantity", "order_placed.lines[].quantity"), UInt16.self, "order_placed.lines[].quantity"), color: SceTestProfile.FromCanonical.enumerated(SceTestProfile.FromCanonical.field(record_1, "color", "order_placed.lines[].color"), [("red", PaintColor.red), ("green", PaintColor.green), ("blue", PaintColor.blue)], "order_placed.lines[].color")) } }, note: SceTestProfile.FromCanonical.optionalField(record_0, "note") { present_0 throws(SceTestProfile.TraceRefusal) in try SceTestProfile.FromCanonical.text(present_0, 32, "order_placed.note") }, priority: SceTestProfile.FromCanonical.optionalField(record_0, "priority") { present_0 throws(SceTestProfile.TraceRefusal) in try SceTestProfile.FromCanonical.integer(present_0, UInt8.self, "order_placed.priority") }, customer: SceTestProfile.FromCanonical.record(SceTestProfile.FromCanonical.field(record_0, "customer", "order_placed.customer"), ["full_name", "vip"], "order_placed.customer") { (record_1: SceRuntime.JSONValue) throws(SceTestProfile.TraceRefusal) -> OrderCustomerPayload in try OrderCustomerPayload(full_name: SceTestProfile.FromCanonical.text(SceTestProfile.FromCanonical.field(record_1, "full_name", "order_placed.customer.full_name"), 24, "order_placed.customer.full_name"), vip: SceTestProfile.FromCanonical.boolean(SceTestProfile.FromCanonical.field(record_1, "vip", "order_placed.customer.vip"), "order_placed.customer.vip")) }, payment: { () throws(SceTestProfile.TraceRefusal) -> OrderPayment in let case_0 = try SceTestProfile.FromCanonical.unionCase(SceTestProfile.FromCanonical.field(record_0, "payment", "order_placed.payment"), ["card", "cash", "voucher"], "order_placed.payment"); switch case_0.variant { case "card": return try OrderPayment.card(SceTestProfile.FromCanonical.record(case_0.value, ["last4", "amount"], "order_placed.payment") { (record_1: SceRuntime.JSONValue) throws(SceTestProfile.TraceRefusal) -> CardPaymentPayload in try CardPaymentPayload(last4: SceTestProfile.FromCanonical.text(SceTestProfile.FromCanonical.field(record_1, "last4", "order_placed.payment.last4"), 4, "order_placed.payment.last4"), amount: SceTestProfile.FromCanonical.integer(SceTestProfile.FromCanonical.field(record_1, "amount", "order_placed.payment.amount"), UInt32.self, "order_placed.payment.amount")) }); case "cash": return try SceTestProfile.FromCanonical.noRecord(case_0.value, "order_placed.payment.cash", OrderPayment.cash); default: return try OrderPayment.voucher(SceTestProfile.FromCanonical.record(case_0.value, ["voucher_code"], "order_placed.payment") { (record_1: SceRuntime.JSONValue) throws(SceTestProfile.TraceRefusal) -> VoucherPaymentPayload in try VoucherPaymentPayload(voucher_code: SceTestProfile.FromCanonical.bytes(SceTestProfile.FromCanonical.field(record_1, "voucher_code", "order_placed.payment.voucher_code"), 4, "order_placed.payment.voucher_code")) }) } }()) }
    }

    /// The record `order_placed` in the canonical form of the test log
    /// (docs/SCE_TEST_PROFILE.md §12.5), which a message of it a trace hands the run carries.
    public static func canonical_order_placed(_ value: OrderPlacedPayload) -> SceRuntime.JSONValue {
        SceTestProfile.Canonical.record(("order_id", SceTestProfile.Canonical.integer(value.order_id)), ("lines", SceTestProfile.Canonical.list(value.lines.map { item_0 -> SceRuntime.JSONValue in SceTestProfile.Canonical.record(("sku", SceTestProfile.Canonical.text(item_0.sku)), ("quantity", SceTestProfile.Canonical.integer(item_0.quantity)), ("color", SceTestProfile.Canonical.enumerated(item_0.color.declaredName))) })), ("note", value.note.map { present_0 -> SceRuntime.JSONValue in SceTestProfile.Canonical.text(present_0) }), ("priority", value.priority.map { present_0 -> SceRuntime.JSONValue in SceTestProfile.Canonical.integer(present_0) }), ("customer", SceTestProfile.Canonical.record(("full_name", SceTestProfile.Canonical.text(value.customer.full_name)), ("vip", SceTestProfile.Canonical.boolean(value.customer.vip)))), ("payment", { () -> SceRuntime.JSONValue in switch value.payment { case OrderPayment.card(let case_0): return SceTestProfile.Canonical.unionCase("card", SceTestProfile.Canonical.record(("last4", SceTestProfile.Canonical.text(case_0.last4)), ("amount", SceTestProfile.Canonical.integer(case_0.amount)))); case OrderPayment.cash: return SceTestProfile.Canonical.unionCase("cash", nil); case OrderPayment.voucher(let case_0): return SceTestProfile.Canonical.unionCase("voucher", SceTestProfile.Canonical.record(("voucher_code", SceTestProfile.Canonical.bytes(case_0.voucher_code)))) } }()))
    }

    /// The record `job_status` read from the canonical form of the test log
    /// (docs/SCE_TEST_PROFILE.md §12.5), as a recorded input trace holds it
    /// (docs/SCE_TTCN3_LANE.md §10.3). The inverse of `canonical_job_status`.
    ///
    /// - Throws: `SceTestProfile.TraceRefusal` naming where its value does not belong to its type.
    public static func read_job_status(_ value: SceRuntime.JSONValue) throws(SceTestProfile.TraceRefusal)
        -> JobStatusPayload
    {
        try SceTestProfile.FromCanonical.record(value, ["elapsed_ms", "note", "codes", "tag", "retry", "count"], "job_status") { (record_0: SceRuntime.JSONValue) throws(SceTestProfile.TraceRefusal) -> JobStatusPayload in try JobStatusPayload(elapsed_ms: SceTestProfile.FromCanonical.integer(SceTestProfile.FromCanonical.field(record_0, "elapsed_ms", "job_status.elapsed_ms"), UInt32.self, "job_status.elapsed_ms"), note: SceTestProfile.FromCanonical.optionalField(record_0, "note") { present_0 throws(SceTestProfile.TraceRefusal) in try SceTestProfile.FromCanonical.text(present_0, 32, "job_status.note") }, codes: SceTestProfile.FromCanonical.elements(SceTestProfile.FromCanonical.field(record_0, "codes", "job_status.codes"), 0, 4, "job_status.codes") { item_0 throws(SceTestProfile.TraceRefusal) in try SceTestProfile.FromCanonical.integer(item_0, UInt8.self, "job_status.codes[]") }, tag: SceTestProfile.FromCanonical.optionalField(record_0, "tag") { present_0 throws(SceTestProfile.TraceRefusal) in try SceTestProfile.FromCanonical.integer(present_0, UInt8.self, "job_status.tag") }, retry: SceTestProfile.FromCanonical.optionalField(record_0, "retry") { present_0 throws(SceTestProfile.TraceRefusal) in try SceTestProfile.FromCanonical.integer(present_0, UInt8.self, "job_status.retry") }, count: SceTestProfile.FromCanonical.integer(SceTestProfile.FromCanonical.field(record_0, "count", "job_status.count"), UInt32.self, "job_status.count")) }
    }

    /// The record `job_status` in the canonical form of the test log
    /// (docs/SCE_TEST_PROFILE.md §12.5), which a message of it a trace hands the run carries.
    public static func canonical_job_status(_ value: JobStatusPayload) -> SceRuntime.JSONValue {
        SceTestProfile.Canonical.record(("elapsed_ms", SceTestProfile.Canonical.integer(value.elapsed_ms)), ("note", value.note.map { present_0 -> SceRuntime.JSONValue in SceTestProfile.Canonical.text(present_0) }), ("codes", SceTestProfile.Canonical.list(value.codes.map { item_0 -> SceRuntime.JSONValue in SceTestProfile.Canonical.integer(item_0) })), ("tag", value.tag.map { present_0 -> SceRuntime.JSONValue in SceTestProfile.Canonical.integer(present_0) }), ("retry", value.retry.map { present_0 -> SceRuntime.JSONValue in SceTestProfile.Canonical.integer(present_0) }), ("count", SceTestProfile.Canonical.integer(value.count)))
    }

    /// The record `closed_enum_form_extras` read from the canonical form of the test log
    /// (docs/SCE_TEST_PROFILE.md §12.5), as a recorded input trace holds it
    /// (docs/SCE_TTCN3_LANE.md §10.3). The inverse of `canonical_closed_enum_form_extras`.
    ///
    /// - Throws: `SceTestProfile.TraceRefusal` naming where its value does not belong to its type.
    public static func read_closed_enum_form_extras(_ value: SceRuntime.JSONValue) throws(SceTestProfile.TraceRefusal)
        -> ClosedEnumFormExtrasPayload
    {
        try SceTestProfile.FromCanonical.record(value, ["slots", "least", "flags", "spare", "grid"], "closed_enum_form_extras") { (record_0: SceRuntime.JSONValue) throws(SceTestProfile.TraceRefusal) -> ClosedEnumFormExtrasPayload in try ClosedEnumFormExtrasPayload(slots: SceTestProfile.FromCanonical.elements(SceTestProfile.FromCanonical.field(record_0, "slots", "closed_enum_form_extras.slots"), 0, 3, "closed_enum_form_extras.slots") { item_0 throws(SceTestProfile.TraceRefusal) in try SceTestProfile.FromCanonical.nullable(item_0) { present_1 throws(SceTestProfile.TraceRefusal) in try SceTestProfile.FromCanonical.integer(present_1, UInt16.self, "closed_enum_form_extras.slots[]") } }, least: SceTestProfile.FromCanonical.integer(SceTestProfile.FromCanonical.field(record_0, "least", "closed_enum_form_extras.least"), Int64.self, "closed_enum_form_extras.least"), flags: SceTestProfile.FromCanonical.elements(SceTestProfile.FromCanonical.field(record_0, "flags", "closed_enum_form_extras.flags"), 1, 2, "closed_enum_form_extras.flags") { item_0 throws(SceTestProfile.TraceRefusal) in try SceTestProfile.FromCanonical.boolean(item_0, "closed_enum_form_extras.flags[]") }, spare: SceTestProfile.FromCanonical.optionalField(record_0, "spare") { present_0 throws(SceTestProfile.TraceRefusal) in try SceTestProfile.FromCanonical.elements(present_0, 0, 2, "closed_enum_form_extras.spare") { item_1 throws(SceTestProfile.TraceRefusal) in try SceTestProfile.FromCanonical.integer(item_1, UInt8.self, "closed_enum_form_extras.spare[]") } }, grid: SceTestProfile.FromCanonical.elements(SceTestProfile.FromCanonical.field(record_0, "grid", "closed_enum_form_extras.grid"), 0, 2, "closed_enum_form_extras.grid") { item_0 throws(SceTestProfile.TraceRefusal) in try SceTestProfile.FromCanonical.elements(item_0, 0, 2, "closed_enum_form_extras.grid[]") { item_1 throws(SceTestProfile.TraceRefusal) in try SceTestProfile.FromCanonical.integer(item_1, UInt8.self, "closed_enum_form_extras.grid[][]") } }) }
    }

    /// The record `closed_enum_form_extras` in the canonical form of the test log
    /// (docs/SCE_TEST_PROFILE.md §12.5), which a message of it a trace hands the run carries.
    public static func canonical_closed_enum_form_extras(_ value: ClosedEnumFormExtrasPayload) -> SceRuntime.JSONValue {
        SceTestProfile.Canonical.record(("slots", SceTestProfile.Canonical.list(value.slots.map { item_0 -> SceRuntime.JSONValue in SceTestProfile.Canonical.optionalElement(item_0.map { present_1 -> SceRuntime.JSONValue in SceTestProfile.Canonical.integer(present_1) }) })), ("least", SceTestProfile.Canonical.integer(value.least)), ("flags", SceTestProfile.Canonical.list(value.flags.map { item_0 -> SceRuntime.JSONValue in SceTestProfile.Canonical.boolean(item_0) })), ("spare", value.spare.map { present_0 -> SceRuntime.JSONValue in SceTestProfile.Canonical.list(present_0.map { item_1 -> SceRuntime.JSONValue in SceTestProfile.Canonical.integer(item_1) }) }), ("grid", SceTestProfile.Canonical.list(value.grid.map { item_0 -> SceRuntime.JSONValue in SceTestProfile.Canonical.list(item_0.map { item_1 -> SceRuntime.JSONValue in SceTestProfile.Canonical.integer(item_1) }) })))
    }

    /// The reader of the inputs of a recorded trace (docs/SCE_TTCN3_LANE.md §10.2,
    /// `SceTestProfile.InputReader`): the message an input of the record type `typeName` on the
    /// system port `systemPort` carries, its value read from the canonical form.
    ///
    /// - Throws: `SceTestProfile.TraceRefusal` naming why, where the configuration has no such system
    ///   port, the `in` list of its type does not name the type, or the value does not belong to it.
    public static func input(systemPort: String, typeName: String, value: SceRuntime.JSONValue)
        throws(SceTestProfile.TraceRefusal) -> SceTestProfile.Message
    {
        switch (systemPort, typeName) {
        case ("sut", "order_placed"):
            return try SceTestProfile.Message.of(
                event: OrderPlaced.eventName, value: read_order_placed(value), typeName: "order_placed",
                canonical: { ClosedEnumFormsConfiguration.canonical_order_placed($0) })
        case ("sut", "job_status"):
            return try SceTestProfile.Message.of(
                event: JobStatus.eventName, value: read_job_status(value), typeName: "job_status",
                canonical: { ClosedEnumFormsConfiguration.canonical_job_status($0) })
        case ("sut", "closed_enum_form_extras"):
            return try SceTestProfile.Message.of(
                event: ClosedEnumFormExtras.eventName, value: read_closed_enum_form_extras(value), typeName: "closed_enum_form_extras",
                canonical: { ClosedEnumFormsConfiguration.canonical_closed_enum_form_extras($0) })
        default:
            throw SceTestProfile.TraceRefusal.notAnInput(systemPort, typeName, ["sut"])
        }
    }
}
