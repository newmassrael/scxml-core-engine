// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What a host says it does with a machine's duties (docs/adr/0038). The
// generated machines refuse a host by comparing against ActionPerforms, so
// the two named values and the value no constant names are pinned here.

package sce

import "testing"

func TestTheTwoRealizationsAreNamedAndOnlyPerformsPerforms(t *testing.T) {
	if ActionPerforms == ActionSimulates {
		t.Fatalf("Performs and Simulates are one value: %d", ActionPerforms)
	}
	if got := ActionPerforms.String(); got != "Performs" {
		t.Errorf("ActionPerforms is named %q", got)
	}
	if got := ActionSimulates.String(); got != "Simulates" {
		t.Errorf("ActionSimulates is named %q", got)
	}
}

func TestARealizationNoConstantNamesIsNotPerforms(t *testing.T) {
	var unset ActionRealization
	if unset == ActionPerforms {
		t.Errorf("the zero value is Performs: a host that declares nothing would pass a refusal made against ActionPerforms")
	}
	if got := ActionRealization(99).String(); got != "Simulates" {
		t.Errorf("a value no constant names reads as %q, not Simulates", got)
	}
}
