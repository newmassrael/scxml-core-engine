// SCE-GENERATED — DO NOT EDIT
// source-hash: 72891728be38ed1db13e4394197bad5a7eaecc0eeac062f21e8fdc061c4ae890
// SCE-MAP: forms_configuration.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.forms_configuration

/**
 * The test configuration `forms_configuration` (docs/SCE_TEST_PROFILE.md §11.6) as the
 * profile's runtime reads it: the MTC's component type and each PTC's, the ports and timers
 * each type declares, each `<sce:connect>` and each `<sce:map>`.
 */
fun configuration(): com.sce.runtime.test.Configuration = com.sce.runtime.test.Configuration(
    mtcType = "forms_listener",
    ptcTypes = emptyMap(),
    portsOfType = mapOf(
        "forms_listener" to listOf("sut"),
    ),
    timersOfType = emptyMap(),
    connections = emptyList(),
    mappings = listOf(
        com.sce.runtime.test.Endpoint("mtc", "sut") to "sut",
    ),
)

/**
 * The machine of each component type of the configuration: the component the profile's
 * runtime drives, generated beside its test component's machine (docs/SCE_TEST_PROFILE.md
 * §12.3). Null for a type the configuration does not name, which the run ends with `error`
 * where it is created.
 */
fun machine(componentType: String): com.sce.runtime.test.ComponentMachine? = when (componentType) {
    "forms_listener" -> com.sce.generated.forms_listener.FormsListenerComponent()
    else -> null
}

/**
 * The module parameters the components of the configuration read
 * (docs/SCE_TEST_PROFILE.md §11.5), each of which a suite that runs it gives a value.
 */
fun moduleParameters(): List<String> = emptyList()

/**
 * The record `order_placed` read from the canonical form of the test log
 * (docs/SCE_TEST_PROFILE.md §12.5), as a recorded input trace holds it (docs/SCE_TTCN3_LANE.md
 * §10.3). The inverse of [canonical_order_placed].
 *
 * @throws com.sce.runtime.test.TraceRefusal naming where its value does not belong to its type.
 */
fun read_order_placed(value: Any?): com.sce.generated.order_placed.OrderPlacedPayload =
    com.sce.runtime.test.FromCanonical.record(value, listOf("order_id", "lines", "note", "priority", "customer", "payment"), "order_placed").let { record_0 -> com.sce.generated.order_placed.OrderPlacedPayload(order_id = com.sce.runtime.test.FromCanonical.integer(com.sce.runtime.test.FromCanonical.field(record_0, "order_id", "order_placed.order_id"), com.sce.runtime.test.FromCanonical.UINT32, "order_placed.order_id"), lines = com.sce.runtime.test.FromCanonical.elements(com.sce.runtime.test.FromCanonical.field(record_0, "lines", "order_placed.lines"), 0, 4, "order_placed.lines").map { item_0 -> com.sce.runtime.test.FromCanonical.record(item_0, listOf("sku", "quantity", "color"), "order_placed.lines[]").let { record_1 -> com.sce.generated.order_line.OrderLinePayload(sku = com.sce.runtime.test.FromCanonical.text(com.sce.runtime.test.FromCanonical.field(record_1, "sku", "order_placed.lines[].sku"), 16, "order_placed.lines[].sku"), quantity = com.sce.runtime.test.FromCanonical.integer(com.sce.runtime.test.FromCanonical.field(record_1, "quantity", "order_placed.lines[].quantity"), com.sce.runtime.test.FromCanonical.UINT16, "order_placed.lines[].quantity"), color = com.sce.runtime.test.FromCanonical.enumerated(com.sce.runtime.test.FromCanonical.field(record_1, "color", "order_placed.lines[].color"), listOf("red" to com.sce.generated.paint_color.PaintColor.RED, "green" to com.sce.generated.paint_color.PaintColor.GREEN, "blue" to com.sce.generated.paint_color.PaintColor.BLUE), "order_placed.lines[].color")) } }, note = com.sce.runtime.test.FromCanonical.optionalField(record_0, "note") { present_0 -> com.sce.runtime.test.FromCanonical.text(present_0, 32, "order_placed.note") }, priority = com.sce.runtime.test.FromCanonical.optionalField(record_0, "priority") { present_0 -> com.sce.runtime.test.FromCanonical.integer(present_0, com.sce.runtime.test.FromCanonical.UINT8, "order_placed.priority") }, customer = com.sce.runtime.test.FromCanonical.record(com.sce.runtime.test.FromCanonical.field(record_0, "customer", "order_placed.customer"), listOf("full_name", "vip"), "order_placed.customer").let { record_1 -> com.sce.generated.order_customer.OrderCustomerPayload(full_name = com.sce.runtime.test.FromCanonical.text(com.sce.runtime.test.FromCanonical.field(record_1, "full_name", "order_placed.customer.full_name"), 24, "order_placed.customer.full_name"), vip = com.sce.runtime.test.FromCanonical.boolean(com.sce.runtime.test.FromCanonical.field(record_1, "vip", "order_placed.customer.vip"), "order_placed.customer.vip")) }, payment = com.sce.runtime.test.FromCanonical.unionCase(com.sce.runtime.test.FromCanonical.field(record_0, "payment", "order_placed.payment"), listOf("card", "cash", "voucher"), "order_placed.payment").let { case_0 -> when (case_0.variant) { "card" -> com.sce.generated.order_payment.OrderPayment.Card(com.sce.runtime.test.FromCanonical.record(case_0.value, listOf("last4", "amount"), "order_placed.payment").let { record_1 -> com.sce.generated.card_payment.CardPaymentPayload(last4 = com.sce.runtime.test.FromCanonical.text(com.sce.runtime.test.FromCanonical.field(record_1, "last4", "order_placed.payment.last4"), 4, "order_placed.payment.last4"), amount = com.sce.runtime.test.FromCanonical.integer(com.sce.runtime.test.FromCanonical.field(record_1, "amount", "order_placed.payment.amount"), com.sce.runtime.test.FromCanonical.UINT32, "order_placed.payment.amount")) }); "cash" -> com.sce.runtime.test.FromCanonical.noRecord(case_0.value, "order_placed.payment.cash", com.sce.generated.order_payment.OrderPayment.Cash); else -> com.sce.generated.order_payment.OrderPayment.Voucher(com.sce.runtime.test.FromCanonical.record(case_0.value, listOf("voucher_code"), "order_placed.payment").let { record_1 -> com.sce.generated.voucher_payment.VoucherPaymentPayload(voucher_code = com.sce.runtime.test.FromCanonical.bytes(com.sce.runtime.test.FromCanonical.field(record_1, "voucher_code", "order_placed.payment.voucher_code"), 4, "order_placed.payment.voucher_code")) }) } }) }

/**
 * The record `order_placed` in the canonical form of the test log
 * (docs/SCE_TEST_PROFILE.md §12.5), which a message of it a trace hands the run carries.
 */
fun canonical_order_placed(value: com.sce.generated.order_placed.OrderPlacedPayload): Any =
    com.sce.runtime.test.Canonical.record("order_id" to com.sce.runtime.test.Canonical.integer(value.order_id), "lines" to com.sce.runtime.test.Canonical.list(value.lines.map { item_0 -> com.sce.runtime.test.Canonical.record("sku" to com.sce.runtime.test.Canonical.text(item_0.sku), "quantity" to com.sce.runtime.test.Canonical.integer(item_0.quantity), "color" to com.sce.runtime.test.Canonical.enumerated(item_0.color.declaredName())) }), "note" to value.note?.let { present_0 -> com.sce.runtime.test.Canonical.text(present_0) }, "priority" to value.priority?.let { present_0 -> com.sce.runtime.test.Canonical.integer(present_0) }, "customer" to com.sce.runtime.test.Canonical.record("full_name" to com.sce.runtime.test.Canonical.text(value.customer.full_name), "vip" to com.sce.runtime.test.Canonical.boolean(value.customer.vip)), "payment" to when (val case_0 = value.payment) { is com.sce.generated.order_payment.OrderPayment.Card -> com.sce.runtime.test.Canonical.unionCase("card", com.sce.runtime.test.Canonical.record("last4" to com.sce.runtime.test.Canonical.text(case_0.value.last4), "amount" to com.sce.runtime.test.Canonical.integer(case_0.value.amount))); is com.sce.generated.order_payment.OrderPayment.Cash -> com.sce.runtime.test.Canonical.unionCase("cash", null); is com.sce.generated.order_payment.OrderPayment.Voucher -> com.sce.runtime.test.Canonical.unionCase("voucher", com.sce.runtime.test.Canonical.record("voucher_code" to com.sce.runtime.test.Canonical.bytes(case_0.value.voucher_code))) })

/**
 * The record `job_status` read from the canonical form of the test log
 * (docs/SCE_TEST_PROFILE.md §12.5), as a recorded input trace holds it (docs/SCE_TTCN3_LANE.md
 * §10.3). The inverse of [canonical_job_status].
 *
 * @throws com.sce.runtime.test.TraceRefusal naming where its value does not belong to its type.
 */
fun read_job_status(value: Any?): com.sce.generated.job_status.JobStatusPayload =
    com.sce.runtime.test.FromCanonical.record(value, listOf("elapsed_ms", "note", "codes", "tag", "retry", "count"), "job_status").let { record_0 -> com.sce.generated.job_status.JobStatusPayload(elapsed_ms = com.sce.runtime.test.FromCanonical.integer(com.sce.runtime.test.FromCanonical.field(record_0, "elapsed_ms", "job_status.elapsed_ms"), com.sce.runtime.test.FromCanonical.UINT32, "job_status.elapsed_ms"), note = com.sce.runtime.test.FromCanonical.optionalField(record_0, "note") { present_0 -> com.sce.runtime.test.FromCanonical.text(present_0, 32, "job_status.note") }, codes = com.sce.runtime.test.FromCanonical.elements(com.sce.runtime.test.FromCanonical.field(record_0, "codes", "job_status.codes"), 0, 4, "job_status.codes").map { item_0 -> com.sce.runtime.test.FromCanonical.integer(item_0, com.sce.runtime.test.FromCanonical.UINT8, "job_status.codes[]") }, tag = com.sce.runtime.test.FromCanonical.optionalField(record_0, "tag") { present_0 -> com.sce.runtime.test.FromCanonical.integer(present_0, com.sce.runtime.test.FromCanonical.UINT8, "job_status.tag") }, retry = com.sce.runtime.test.FromCanonical.optionalField(record_0, "retry") { present_0 -> com.sce.runtime.test.FromCanonical.integer(present_0, com.sce.runtime.test.FromCanonical.UINT8, "job_status.retry") }, count = com.sce.runtime.test.FromCanonical.integer(com.sce.runtime.test.FromCanonical.field(record_0, "count", "job_status.count"), com.sce.runtime.test.FromCanonical.UINT32, "job_status.count")) }

/**
 * The record `job_status` in the canonical form of the test log
 * (docs/SCE_TEST_PROFILE.md §12.5), which a message of it a trace hands the run carries.
 */
fun canonical_job_status(value: com.sce.generated.job_status.JobStatusPayload): Any =
    com.sce.runtime.test.Canonical.record("elapsed_ms" to com.sce.runtime.test.Canonical.integer(value.elapsed_ms), "note" to value.note?.let { present_0 -> com.sce.runtime.test.Canonical.text(present_0) }, "codes" to com.sce.runtime.test.Canonical.list(value.codes.map { item_0 -> com.sce.runtime.test.Canonical.integer(item_0) }), "tag" to value.tag?.let { present_0 -> com.sce.runtime.test.Canonical.integer(present_0) }, "retry" to value.retry?.let { present_0 -> com.sce.runtime.test.Canonical.integer(present_0) }, "count" to com.sce.runtime.test.Canonical.integer(value.count))

/**
 * The record `form_extras` read from the canonical form of the test log
 * (docs/SCE_TEST_PROFILE.md §12.5), as a recorded input trace holds it (docs/SCE_TTCN3_LANE.md
 * §10.3). The inverse of [canonical_form_extras].
 *
 * @throws com.sce.runtime.test.TraceRefusal naming where its value does not belong to its type.
 */
fun read_form_extras(value: Any?): com.sce.generated.form_extras.FormExtrasPayload =
    com.sce.runtime.test.FromCanonical.record(value, listOf("shape", "slots", "least", "flags", "spare", "grid"), "form_extras").let { record_0 -> com.sce.generated.form_extras.FormExtrasPayload(shape = com.sce.runtime.test.FromCanonical.openEnumerated(com.sce.runtime.test.FromCanonical.field(record_0, "shape", "form_extras.shape"), listOf("dot" to com.sce.generated.shape_kind_open.ShapeKindOpen.DOT, "circle" to com.sce.generated.shape_kind_open.ShapeKindOpen.CIRCLE, "rect" to com.sce.generated.shape_kind_open.ShapeKindOpen.RECT), com.sce.runtime.test.FromCanonical.UINT8, "form_extras.shape") { raw -> com.sce.generated.shape_kind_open.ShapeKindOpen.fromUnderlying(raw) }, slots = com.sce.runtime.test.FromCanonical.elements(com.sce.runtime.test.FromCanonical.field(record_0, "slots", "form_extras.slots"), 0, 3, "form_extras.slots").map { item_0 -> com.sce.runtime.test.FromCanonical.nullable(item_0) { present_1 -> com.sce.runtime.test.FromCanonical.integer(present_1, com.sce.runtime.test.FromCanonical.UINT16, "form_extras.slots[]") } }, least = com.sce.runtime.test.FromCanonical.integer(com.sce.runtime.test.FromCanonical.field(record_0, "least", "form_extras.least"), com.sce.runtime.test.FromCanonical.INT64, "form_extras.least"), flags = com.sce.runtime.test.FromCanonical.elements(com.sce.runtime.test.FromCanonical.field(record_0, "flags", "form_extras.flags"), 1, 2, "form_extras.flags").map { item_0 -> com.sce.runtime.test.FromCanonical.boolean(item_0, "form_extras.flags[]") }, spare = com.sce.runtime.test.FromCanonical.optionalField(record_0, "spare") { present_0 -> com.sce.runtime.test.FromCanonical.elements(present_0, 0, 2, "form_extras.spare").map { item_1 -> com.sce.runtime.test.FromCanonical.integer(item_1, com.sce.runtime.test.FromCanonical.UINT8, "form_extras.spare[]") } }, grid = com.sce.runtime.test.FromCanonical.elements(com.sce.runtime.test.FromCanonical.field(record_0, "grid", "form_extras.grid"), 0, 2, "form_extras.grid").map { item_0 -> com.sce.runtime.test.FromCanonical.elements(item_0, 0, 2, "form_extras.grid[]").map { item_1 -> com.sce.runtime.test.FromCanonical.integer(item_1, com.sce.runtime.test.FromCanonical.UINT8, "form_extras.grid[][]") } }) }

/**
 * The record `form_extras` in the canonical form of the test log
 * (docs/SCE_TEST_PROFILE.md §12.5), which a message of it a trace hands the run carries.
 */
fun canonical_form_extras(value: com.sce.generated.form_extras.FormExtrasPayload): Any =
    com.sce.runtime.test.Canonical.record("shape" to com.sce.runtime.test.Canonical.openEnumerated(value.shape.declaredName(), com.sce.runtime.test.Canonical.integer(value.shape.toUnderlying())), "slots" to com.sce.runtime.test.Canonical.list(value.slots.map { item_0 -> com.sce.runtime.test.Canonical.optionalElement(item_0) { present_1 -> com.sce.runtime.test.Canonical.integer(present_1) } }), "least" to com.sce.runtime.test.Canonical.integer(value.least), "flags" to com.sce.runtime.test.Canonical.list(value.flags.map { item_0 -> com.sce.runtime.test.Canonical.boolean(item_0) }), "spare" to value.spare?.let { present_0 -> com.sce.runtime.test.Canonical.list(present_0.map { item_1 -> com.sce.runtime.test.Canonical.integer(item_1) }) }, "grid" to com.sce.runtime.test.Canonical.list(value.grid.map { item_0 -> com.sce.runtime.test.Canonical.list(item_0.map { item_1 -> com.sce.runtime.test.Canonical.integer(item_1) }) }))

/**
 * The reader of the inputs of a recorded trace (docs/SCE_TTCN3_LANE.md §10.2,
 * `com.sce.runtime.test.InputReader`): the message an input of the record type [typeName] on the
 * system port [systemPort] carries, its value read from the canonical form.
 *
 * @throws com.sce.runtime.test.TraceRefusal naming why, where the configuration has no such system
 *   port, the `in` list of its type does not name the type, or the value does not belong to it.
 */
fun input(systemPort: String, typeName: String, value: Any?): com.sce.runtime.test.Message = when (systemPort to typeName) {
    "sut" to "order_placed" -> com.sce.runtime.test.Message.of(com.sce.generated.order_placed.EVENT_NAME, read_order_placed(value), "order_placed", ::canonical_order_placed)
    "sut" to "job_status" -> com.sce.runtime.test.Message.of(com.sce.generated.job_status.EVENT_NAME, read_job_status(value), "job_status", ::canonical_job_status)
    "sut" to "form_extras" -> com.sce.runtime.test.Message.of(com.sce.generated.form_extras.EVENT_NAME, read_form_extras(value), "form_extras", ::canonical_form_extras)
    else -> throw com.sce.runtime.test.TraceRefusal.notAnInput(systemPort, typeName, listOf("sut"))
}
