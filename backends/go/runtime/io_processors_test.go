// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import "testing"

func locationOf(descriptors []IoProcessorDescriptor, name string) string {
	for _, d := range descriptors {
		if d.Name == name {
			return d.Location
		}
	}
	return ""
}

func TestScxmlProcessorIsPublishedUnderBothSpellings(t *testing.T) {
	descriptors := BuildIoProcessors("session-1", "")

	if got, want := locationOf(descriptors, ScxmlProcessorAlias), locationOf(descriptors, SCXMLEventProcessorType); got != want {
		t.Fatalf("alias location %q != entry-name location %q", got, want)
	}
	if got := locationOf(descriptors, ScxmlProcessorAlias); got != "sce://scxml/session-1" {
		t.Fatalf("scxml location = %q, want sce://scxml/session-1", got)
	}
}

func TestAPublishedLocationReadsBackAsTheSessionItNames(t *testing.T) {
	// The round trip is the clause: an origin that cannot be decoded back to a
	// session is not an address a peer can answer.
	for _, sessionID := range []string{"session-1", "a b/c#d", "parent.state.1.inv_peer"} {
		if got := SessionIDFromScxmlLocation(ScxmlProcessorLocation(sessionID)); got != sessionID {
			t.Errorf("round trip of %q gave %q", sessionID, got)
		}
	}
	for _, notALocation := range []string{"sce://scxml/", "session-1", "http://host/x", ""} {
		if got := SessionIDFromScxmlLocation(notALocation); got != "" {
			t.Errorf("SessionIDFromScxmlLocation(%q) = %q, want empty", notALocation, got)
		}
	}
}

func TestPublishedOriginIsTheLocationTheSenderPublishes(t *testing.T) {
	descriptors := BuildIoProcessors("session-1", "")

	if got, want := PublishedOrigin("session-1"), locationOf(descriptors, ScxmlProcessorAlias); got != want {
		t.Fatalf("PublishedOrigin = %q, published location = %q", got, want)
	}
}

func TestPublishedOriginPassesAnAddressThroughAndKeepsEmptyEmpty(t *testing.T) {
	// A remote child is stamped with a URI, not an id; wrapping it again would
	// produce an address naming nothing.
	if got := PublishedOrigin("sce://mesh/peer-7"); got != "sce://mesh/peer-7" {
		t.Errorf("PublishedOrigin passed-through address = %q", got)
	}
	if got := PublishedOrigin(""); got != "" {
		t.Errorf("PublishedOrigin(\"\") = %q, want empty", got)
	}
}
