// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import (
	"encoding/json"
	"os"
	"testing"
)

const reservedTypeTable = "../../../sce-build/tests/fixtures/host_processor/reserved_type_cases.json"

// panics reports whether register panicked.
func panics(register func()) (panicked bool) {
	defer func() { panicked = recover() != nil }()
	register()
	return false
}

// TestRegistrationRefusesTheReservedTypesTheSharedTableNames asks every case of
// the table the build's declaration check and every runtime's registration
// read — of both registrations, not only of the predicate, so a registration
// that stopped consulting it fails here.
func TestRegistrationRefusesTheReservedTypesTheSharedTableNames(t *testing.T) {
	text, err := os.ReadFile(reservedTypeTable)
	if err != nil {
		t.Fatalf("read %s: %v", reservedTypeTable, err)
	}
	var table struct {
		Cases []struct {
			Type     string `json:"type"`
			Reserved bool   `json:"reserved"`
		} `json:"cases"`
	}
	if err := json.Unmarshal(text, &table); err != nil {
		t.Fatalf("the table is not JSON: %v", err)
	}
	// A floor: an empty table would pass every assertion below.
	if len(table.Cases) < 10 {
		t.Fatalf("the table lost cases: %d", len(table.Cases))
	}
	for _, c := range table.Cases {
		if got := IsReservedType(c.Type); got != c.Reserved {
			t.Errorf("IsReservedType(%q) = %v, want %v", c.Type, got, c.Reserved)
		}
		send := panics(func() {
			var e Engine[int, int]
			e.RegisterEventProcessor(c.Type, func(HostSendRequest) []HostSendResponse { return nil })
		})
		invoke := panics(func() {
			var e Engine[int, int]
			e.RegisterInvoker(c.Type, func(HostInvokeEvent) *HostInvokeResponse { return nil })
		})
		if send != c.Reserved || invoke != c.Reserved {
			t.Errorf("%q: RegisterEventProcessor refused=%v, RegisterInvoker refused=%v, want %v",
				c.Type, send, invoke, c.Reserved)
		}
	}
}

// TestAMeshRouterIsRegisteredThroughItsOwnDoor: the router's door serves the
// type the general one refuses.
func TestAMeshRouterIsRegisteredThroughItsOwnDoor(t *testing.T) {
	var e Engine[int, int]
	if e.HasEventProcessor(MeshProcessorType) {
		t.Fatalf("a fresh engine already serves %s", MeshProcessorType)
	}
	e.RegisterMeshRouter(func(HostSendRequest) []HostSendResponse { return nil })
	if !e.HasEventProcessor(MeshProcessorType) {
		t.Fatalf("RegisterMeshRouter did not serve %s", MeshProcessorType)
	}
}
