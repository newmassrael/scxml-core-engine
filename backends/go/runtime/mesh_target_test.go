// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import (
	"encoding/json"
	"os"
	"testing"
)

const meshTargetTable = "../../../tests/mesh/mesh_target_cases.json"

// TestAMeshPeerIsNamedAsTheSharedTableNamesIt holds MeshPeer to
// tests/mesh/mesh_target_cases.json, the table the C++ core's
// SendHelper::isMeshTarget, the build and the other runtimes read too.
func TestAMeshPeerIsNamedAsTheSharedTableNamesIt(t *testing.T) {
	text, err := os.ReadFile(meshTargetTable)
	if err != nil {
		t.Fatalf("read %s: %v", meshTargetTable, err)
	}
	var table struct {
		Cases []struct {
			Target string  `json:"target"`
			Peer   *string `json:"peer"`
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
		peer, ok := MeshPeer(c.Target)
		if ok != (c.Peer != nil) || (ok && peer != *c.Peer) {
			t.Errorf("MeshPeer(%q) = (%q, %v), want %v", c.Target, peer, ok, c.Peer)
		}
		if IsMeshTarget(c.Target) != (c.Peer != nil) {
			t.Errorf("IsMeshTarget(%q) disagrees with the table", c.Target)
		}
	}
}
