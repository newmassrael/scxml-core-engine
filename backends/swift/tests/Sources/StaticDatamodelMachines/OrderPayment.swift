// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: order_payment.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="union")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// A value of `order_payment`: exactly one case, told apart by its tag `kind`, a value of
/// ``PaymentKind``. Two values are equal when they are in one case and their records are equal.
public enum OrderPayment: Equatable, Sendable {
    /// The revision of the union: its tag enum's, `payment_kind`'s (a union declares none of its own).
    public static let revision: UInt32 = 1

    /// The most bytes a value's JSON text takes: its largest case, every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 70

    /// The id of the union's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:order_payment:1"

    /// A value as a JSON Schema (2020-12): the `oneOf` of the cases, each its tag `kind` beside the
    /// fields of its record, every bound as this build resolved it.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:order_payment:1\",\"x-sce-source-hash\":\"d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"card\"},\"last4\":{\"type\":\"string\",\"maxLength\":4,\"x-sce-max-bytes\":4},\"amount\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}},\"required\":[\"kind\",\"last4\",\"amount\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"cash\"}},\"required\":[\"kind\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"voucher\"},\"voucher_code\":{\"type\":\"string\",\"maxLength\":4,\"pattern\":\"^[\\\\u0000-\\\\u00ff]*$\"}},\"required\":[\"kind\",\"voucher_code\"],\"additionalProperties\":false}]}"

    /// The case `card`, holding its record.
    case card(CardPaymentPayload)

    /// The case `cash`, its tag alone.
    case cash

    /// The case `voucher`, holding its record.
    case voucher(VoucherPaymentPayload)

    /// The case this value is in: the tag its text writes under `kind`.
    public var kind: PaymentKind {
        switch self {
        case .card:
            return PaymentKind.card
        case .cash:
            return PaymentKind.cash
        case .voucher:
            return PaymentKind.voucher
        }
    }

    /// The record of the case `card`, read where a test of the tag proved the value is in it.
    public var provenCard: CardPaymentPayload {
        guard case .card(let value) = self else {
            preconditionFailure("a order_payment read as card is in \(kind.declaredName)")
        }
        return value
    }

    /// The record of the case `voucher`, read where a test of the tag proved the value is in it.
    public var provenVoucher: VoucherPaymentPayload {
        guard case .voucher(let value) = self else {
            preconditionFailure("a order_payment read as voucher is in \(kind.declaredName)")
        }
        return value
    }
}
