// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: payment_kind.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package payment_kind

import "github.com/newmassrael/sce-forge-runtime/optional"

type PaymentKind uint8

const (
	PaymentKindCard PaymentKind = 0
	PaymentKindCash PaymentKind = 1
	PaymentKindVoucher PaymentKind = 2
)

// ToUnderlying returns the carrier value this holds.
func (v PaymentKind) ToUnderlying() uint8 {
	return uint8(v)
}

// DeclaredName returns the name the document declares for this value, and the
// absent value for a value no variant names — one the type can hold whether or
// not its set is open, since a defined integer type holds every carrier value.
// It is an optional value as every generated answer that may be absent is
// (docs/adr/0055, section 5).
func (v PaymentKind) DeclaredName() optional.Of[string] {
	switch v {
	case PaymentKindCard:
		return optional.Some("card")
	case PaymentKindCash:
		return optional.Some("cash")
	case PaymentKindVoucher:
		return optional.Some("voucher")
	}
	return optional.None[string]()
}

// PaymentKindFromUnderlying returns the variant raw declares, and the absent
// value when raw declares none — the declared set is closed.
func PaymentKindFromUnderlying(raw uint8) optional.Of[PaymentKind] {
	switch raw {
	case 0:
		return optional.Some(PaymentKindCard)
	case 1:
		return optional.Some(PaymentKindCash)
	case 2:
		return optional.Some(PaymentKindVoucher)
	}
	return optional.None[PaymentKind]()
}
