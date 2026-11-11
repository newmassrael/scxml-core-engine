// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: order_placed.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: order.placed
// Do not edit — regenerate from the source SCXML file.

package order_placed
import (
	"github.com/newmassrael/sce-forge-runtime/optional"
	sce_order_line "github.com/newmassrael/sce-go-tests/integration/static_datamodel/order_line"
	sce_order_customer "github.com/newmassrael/sce-go-tests/integration/static_datamodel/order_customer"
	sce_order_payment "github.com/newmassrael/sce-go-tests/integration/static_datamodel/order_payment"
)

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "order.placed"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// LinesMaxCount is the most elements `lines` holds (the bound its type writes), as this build resolved it.
const LinesMaxCount uint32 = 4

// NoteMaxSize is the most UTF-8 bytes `note` holds (the bound its type writes), as this build resolved it.
const NoteMaxSize uint32 = 32

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 1076

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:order_placed:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:order_placed:1\",\"x-sce-source-hash\":\"d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"order_id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"lines\":{\"type\":\"array\",\"items\":{\"$ref\":\"#/$defs/order_line\"},\"maxItems\":4},\"note\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":32,\"x-sce-max-bytes\":32},{\"type\":\"null\"}]},\"priority\":{\"anyOf\":[{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},{\"type\":\"null\"}]},\"customer\":{\"$ref\":\"#/$defs/order_customer\"},\"payment\":{\"$ref\":\"#/$defs/order_payment\"}},\"required\":[\"order_id\",\"lines\",\"note\",\"priority\",\"customer\",\"payment\"],\"additionalProperties\":false,\"$defs\":{\"order_customer\":{\"type\":\"object\",\"properties\":{\"full_name\":{\"type\":\"string\",\"maxLength\":24,\"x-sce-max-bytes\":24},\"vip\":{\"type\":\"boolean\"}},\"required\":[\"full_name\",\"vip\"],\"additionalProperties\":false},\"order_line\":{\"type\":\"object\",\"properties\":{\"sku\":{\"type\":\"string\",\"maxLength\":16,\"x-sce-max-bytes\":16},\"quantity\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"color\":{\"enum\":[\"red\",\"green\",\"blue\"]}},\"required\":[\"sku\",\"quantity\",\"color\"],\"additionalProperties\":false},\"order_payment\":{\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"card\"},\"last4\":{\"type\":\"string\",\"maxLength\":4,\"x-sce-max-bytes\":4},\"amount\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}},\"required\":[\"kind\",\"last4\",\"amount\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"cash\"}},\"required\":[\"kind\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"voucher\"},\"voucher_code\":{\"type\":\"string\",\"maxLength\":4,\"pattern\":\"^[\\\\u0000-\\\\u00ff]*$\"}},\"required\":[\"kind\",\"voucher_code\"],\"additionalProperties\":false}]}}}"

type OrderPlacedPayload struct {
	OrderId uint32
	Lines []sce_order_line.OrderLinePayload
	Note optional.Of[string]
	Priority optional.Of[uint8]
	Customer sce_order_customer.OrderCustomerPayload
	Payment sce_order_payment.OrderPayment
}
