// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// A Go machine sends a composed value by its name (docs/adr/0086, section
// 5.2), run with no script engine, as the Rust and Kotlin machines do
// (backends/rust/tests/tests/a_composed_value_is_sent_by_its_name.rs,
// AComposedValueIsSentByItsNameTest.kt). The committed machine under
// `static_composed_send/` is generated from
// `sce-build/tests/fixtures/static_datamodel/static_composed_send.scxml`
// (regen: `scripts/regen_static_datamodel_go.sh`).
//
// The first delivery of `batch.sent` is forwarded to the machine itself: four
// lists and an optional list named as the parts of the payload they are, and a
// list variable the machine took the payload's `late` into. The delivery that
// comes back is lifted as any delivery is (section 4), so it is refused whole
// if anything written is outside its type; it is not, and every value read
// from it at depths 1 to 3 is what the first delivery held.

package static_datamodel

import (
	"reflect"
	"testing"

	"github.com/newmassrael/sce-forge-runtime/optional"
	sce "github.com/newmassrael/sce-go-runtime"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_composed_send"
)

func TestADeliveryForwardedByItsNamesComesBackWhole(t *testing.T) {
	policy := static_composed_send.NewStaticComposedSendPolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[static_composed_send.StaticComposedSendState, static_composed_send.StaticComposedSendEvent](&policy)
	if err := engine.Initialize(); err != nil {
		t.Fatal(err)
	}
	// Every list full, the second item's `tags` holding three, an element of
	// `maybe` absent and `spare` present.
	engine.RaiseExternalByName("batch.sent", composedBatch(4, 3, 4))
	settleComposed(&policy, engine.Step, engine.Tick)
	if got := policy.Refusals(); got != 0 {
		t.Errorf("the delivery the machine sent itself was refused %d times", got)
	}
	if got := policy.Deliveries(); got != 2 {
		t.Errorf("deliveries is %d: the forwarded delivery did not come back on the internal queue", got)
	}
	// Depth 1: the lengths of a list of records and a list of enum values.
	if got := policy.ItemCount(); got != 3 {
		t.Errorf("item_count is %d, not 3", got)
	}
	if got := policy.LevelCount(); got != 2 {
		t.Errorf("level_count is %d, not 2", got)
	}
	// Depth 2: each item's list, and a field of an element read by its index.
	if got := policy.TagCount(); got != 11 {
		t.Errorf("tag_count is %d, not 11", got)
	}
	if got := policy.IdSum(); got != 6 {
		t.Errorf("id_sum is %d, not 6", got)
	}
	// A list of lists, walked at both depths.
	if got := policy.CellSum(); got != 10 {
		t.Errorf("cell_sum is %d, not 10", got)
	}
	// An element that may be absent, present.
	if got := policy.FirstMaybe(); !got.Equal(optional.Some[uint16](7)) {
		t.Errorf("first_maybe is %v, not 7", got)
	}
	// The list variable, sent by its name.
	if got := policy.LateSum(); got != 11 {
		t.Errorf("late_sum is %d, not 11", got)
	}
	if got := policy.Late(); !reflect.DeepEqual(got, []uint8{5, 6}) {
		t.Errorf("late is %v, not [5 6]", got)
	}
}
