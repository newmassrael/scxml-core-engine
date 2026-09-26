// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Every runtime reads a host-run invocation's deadline by one grammar, held to
// one table. strconv would not do: ParseFloat reads hex floats, and a deadline
// this backend honours while another refuses it makes a document depend on
// where it was compiled.

package sce

import (
	"encoding/json"
	"math"
	"os"
	"strconv"
	"testing"
)

const deadlineTable = "../../../sce-build/tests/fixtures/host_processor/host_invoke_deadline_values.json"

func TestADeadlineIsReadByTheSharedTable(t *testing.T) {
	text, err := os.ReadFile(deadlineTable)
	if err != nil {
		t.Fatalf("read %s: %v", deadlineTable, err)
	}
	var table struct {
		Accepted [][2]string `json:"accepted"`
		Refused  []string    `json:"refused"`
	}
	if err := json.Unmarshal(text, &table); err != nil {
		t.Fatalf("the table is not JSON: %v", err)
	}
	// A floor: an empty table would pass every assertion below.
	if len(table.Accepted) == 0 || len(table.Refused) == 0 {
		t.Fatalf("the table is empty")
	}
	for _, pair := range table.Accepted {
		want, err := strconv.ParseInt(pair[1], 10, 64)
		if err != nil {
			t.Fatalf("milliseconds %q are not a number: %v", pair[1], err)
		}
		if got, ok := ParseHostInvokeDeadlineMs(pair[0]); !ok || got != want {
			t.Errorf("%q: got (%d, %v), want (%d, true)", pair[0], got, ok, want)
		}
	}
	for _, written := range table.Refused {
		if got, ok := ParseHostInvokeDeadlineMs(written); ok {
			t.Errorf("%q was accepted as %d", written, got)
		}
	}
}

// The largest deadline the grammar admits must not wrap into the past, where
// it would end the invocation the moment it started.
func TestTheLargestDeadlineDoesNotWrap(t *testing.T) {
	if got := saturatingAddMs(1000, math.MaxInt64); got != math.MaxInt64 {
		t.Errorf("got %d, want MaxInt64", got)
	}
	if got := saturatingAddMs(1000, 50); got != 1050 {
		t.Errorf("got %d, want 1050", got)
	}
}
