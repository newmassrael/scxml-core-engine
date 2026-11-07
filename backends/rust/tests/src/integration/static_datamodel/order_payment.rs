// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
#![doc = "SCE-MAP: order_payment.scxml:4 :: _forge_body"]
// SCE-MAP: order_payment.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="union")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use super::card_payment;
use super::payment_kind;
use super::voucher_payment;

/// The revision of the union: its tag enum's, `payment_kind`'s (a union declares none of its own).
#[allow(dead_code)]
pub const REVISION: u32 = 1;
/// The most bytes a value's JSON text takes: its largest case, every list full,
/// every text at its bound, every number at its longest spelling.
#[allow(dead_code)]
pub const DATA_MAX_LEN: u64 = 70;
/// The id of the union's schema, `urn:sce:schema:<document>:<revision>`: the
/// `$id` of `JSON_SCHEMA`.
#[allow(dead_code)]
pub const SCHEMA_ID: &str = "urn:sce:schema:order_payment:1";
/// A value as a JSON Schema (2020-12): the `oneOf` of the cases, each its tag
/// `kind` beside the fields of its record, every bound as this build resolved it.
#[allow(dead_code)]
pub const JSON_SCHEMA: &str = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:order_payment:1\",\"x-sce-source-hash\":\"d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"card\"},\"last4\":{\"type\":\"string\",\"maxLength\":4,\"x-sce-max-bytes\":4},\"amount\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}},\"required\":[\"kind\",\"last4\",\"amount\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"cash\"}},\"required\":[\"kind\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"voucher\"},\"voucher_code\":{\"type\":\"string\",\"maxLength\":4,\"pattern\":\"^[\\\\u0000-\\\\u00ff]*$\"}},\"required\":[\"kind\",\"voucher_code\"],\"additionalProperties\":false}]}";

/// A value of `order_payment`: exactly one case, told apart by its tag `kind`, a
/// value of `payment_kind::PaymentKind`. Two values are equal when they are in one case and
/// their records are equal.
#[derive(Debug, Clone, PartialEq)]
pub enum OrderPayment {
    /// The case `card`, holding its record.
    Card(card_payment::CardPaymentPayload),
    /// The case `cash`, its tag alone.
    Cash,
    /// The case `voucher`, holding its record.
    Voucher(voucher_payment::VoucherPaymentPayload),
}

#[allow(dead_code)]
impl OrderPayment {
    /// The case this value is in: the tag its text writes under `kind`.
    pub fn kind(&self) -> payment_kind::PaymentKind {
        match self {
            Self::Card(_) => payment_kind::PaymentKind::Card,
            Self::Cash => payment_kind::PaymentKind::Cash,
            Self::Voucher(_) => payment_kind::PaymentKind::Voucher,
        }
    }

    /// The record of the case `card`, read where a test of the tag proved
    /// the value is in it.
    pub fn proven_card(&self) -> &card_payment::CardPaymentPayload {
        match self {
            Self::Card(record) => record,
            _ => unreachable!(
                "a value of {} read as {} is in another case",
                "order_payment", "card"
            ),
        }
    }

    /// The record of the case `voucher`, read where a test of the tag proved
    /// the value is in it.
    pub fn proven_voucher(&self) -> &voucher_payment::VoucherPaymentPayload {
        match self {
            Self::Voucher(record) => record,
            _ => unreachable!(
                "a value of {} read as {} is in another case",
                "order_payment", "voucher"
            ),
        }
    }
}
