// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A field a later revision of an event schema added is read out of the
// schema's defaults where a delivery lacks it, and a field of the first
// revision is still required (docs/SCE_ACCEPTED_SUBSET.md, "An event schema
// that grows"). `static_schema_revision` holds the generated machines to the
// same answers; the cases here pin the reader itself.

package sce

import (
	"strings"
	"testing"
)

const revisedDefaults = `{"layout":"week","count":"7","urgent":true}`

func TestAFieldTheDeliveryLacksIsReadOutOfTheDefaults(t *testing.T) {
	fields, err := LiftPayloadWithDefaults(`{"level": 4}`, revisedDefaults)
	if err != nil {
		t.Fatalf("a delivery of the first revision did not read: %v", err)
	}
	if level, err := PayloadUnsigned[uint8](fields, "level"); err != nil || level != 4 {
		t.Errorf("level = %v, %v; want 4", level, err)
	}
	if layout, err := PayloadString(fields, "layout"); err != nil || layout != "week" {
		t.Errorf("layout = %q, %v; want the default", layout, err)
	}
	if count, err := PayloadUnsigned64(fields, "count"); err != nil || count != 7 {
		t.Errorf("count = %v, %v; want the default", count, err)
	}
	if urgent, err := PayloadBool(fields, "urgent"); err != nil || !urgent {
		t.Errorf("urgent = %v, %v; want the default", urgent, err)
	}
}

func TestAFieldTheDeliveryCarriesIsReadAsItCame(t *testing.T) {
	fields, err := LiftPayloadWithDefaults(
		`{"level": 4, "layout": "day", "urgent": false, "source": {"a": [1]}}`, revisedDefaults)
	if err != nil {
		t.Fatalf("a delivery of a later revision did not read: %v", err)
	}
	if layout, err := PayloadString(fields, "layout"); err != nil || layout != "day" {
		t.Errorf("layout = %q, %v; want what the delivery carried", layout, err)
	}
	if urgent, err := PayloadBool(fields, "urgent"); err != nil || urgent {
		t.Errorf("urgent = %v, %v; want what the delivery carried", urgent, err)
	}
	fields, err = LiftPayloadWithDefaults(`{"level": 4, "urgent": 1}`, revisedDefaults)
	if err != nil {
		t.Fatalf("the object did not read: %v", err)
	}
	if _, err := PayloadBool(fields, "urgent"); err == nil {
		t.Errorf("a number was read as a truth value because the field has a default")
	}
}

func TestAFieldOfTheFirstRevisionIsStillRequired(t *testing.T) {
	fields, err := LiftPayloadWithDefaults(`{"layout": "day"}`, revisedDefaults)
	if err != nil {
		t.Fatalf("the object did not read: %v", err)
	}
	if _, err := PayloadUnsigned[uint8](fields, "level"); err == nil || !strings.Contains(err.Error(), `"level"`) {
		t.Errorf("a missing field of the first revision was not refused by name: %v", err)
	}
	fields, err = LiftPayloadWithDefaults("", revisedDefaults)
	if err != nil {
		t.Fatalf("no data did not read as no field: %v", err)
	}
	if _, err := PayloadUnsigned[uint8](fields, "level"); err == nil {
		t.Errorf("no data stood in for a field of the first revision")
	}
	if urgent, err := PayloadBool(fields, "urgent"); err != nil || !urgent {
		t.Errorf("urgent = %v, %v; want the default", urgent, err)
	}
	if _, err := LiftPayloadWithDefaults("5", revisedDefaults); err == nil {
		t.Errorf("a bare value was read as an object of named fields")
	}
}
