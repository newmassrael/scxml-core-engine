// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

func TestAReservedCharacterSurvivesTheRoundTrip(t *testing.T) {
	// `%` is the escape itself, and `:` and `?` would read as URI structure: a
	// session id holding them must still be the one its location names.
	for _, sessionID := range []string{"50%off", "a:b?c=d", "x/y#z"} {
		if got := SessionIDFromScxmlLocation(ScxmlProcessorLocation(sessionID)); got != sessionID {
			t.Errorf("round trip of %q gave %q", sessionID, got)
		}
	}
}

func TestTheScxmlOriginIsTheProcessorAndTheLocationTheSenderPublishes(t *testing.T) {
	descriptors := BuildIoProcessors("a b", "")

	origin := ScxmlOrigin("a b")
	if origin.Processor() != SCXMLEventProcessorType {
		t.Errorf("processor = %q, want %q", origin.Processor(), SCXMLEventProcessorType)
	}
	if got, want := origin.Address(), locationOf(descriptors, ScxmlProcessorAlias); got != want {
		t.Errorf("address = %q, published location = %q", got, want)
	}
	if ScxmlOriginOf("") != nil {
		t.Errorf("a machine with no session id sent an event with an origin")
	}
	if !ScxmlOriginOf("a b").Equal(origin) {
		t.Errorf("ScxmlOriginOf(\"a b\") is not ScxmlOrigin(\"a b\")")
	}
}

func TestAnEventNoProcessorStampedReadsBothFieldsEmpty(t *testing.T) {
	posted := ExternalMetadata("")
	if posted.Origin != nil || posted.OriginAddress() != "" || posted.OriginProcessor() != "" {
		t.Errorf("an event no processor stamped carries an origin: %+v", posted.Origin)
	}
	sent := SentBySession("s-1", "a b")
	if !sent.Origin.Equal(ScxmlOrigin("a b")) || sent.Sender != "a b" || sent.SendID != "s-1" {
		t.Errorf("a session's send carries %+v from %q", sent.Origin, sent.Sender)
	}
	if anonymous := SentBySession("", ""); anonymous.Origin != nil || anonymous.Sender != "" {
		t.Errorf("a machine with no session id sent an origin %+v", anonymous.Origin)
	}
}

func TestAStoredOriginIsTheSenderForASessionAndTheAddressOtherwise(t *testing.T) {
	if got := StoredOriginText(ScxmlOrigin("s1"), "s1"); got != "s1" {
		t.Errorf("format 2 stores the sending session's id, got %q", got)
	}
	if got := StoredOriginText(NewEventOrigin("x-sce-host", "motor"), ""); got != "motor" {
		t.Errorf("a host's pair is stored by its address, got %q", got)
	}
	if got := StoredOriginText(nil, ""); got != "" {
		t.Errorf("no origin is stored as %q", got)
	}
}

func TestAStoredOriginIsReadByTheTableOfSection7(t *testing.T) {
	cases := []struct {
		origin, originType string
		want               *EventOrigin
		sender             string
	}{
		{"", "", nil, ""},
		// An empty address is none, whatever type format 2 saved beside it.
		{"", SCXMLEventProcessorType, nil, ""},
		{"s1", SCXMLEventProcessorType, ScxmlOrigin("s1"), "s1"},
		// An address saved with no type is the SCXML processor's.
		{"s1", "", ScxmlOrigin("s1"), "s1"},
		{"mesh://ecu", "", NewEventOrigin(SCXMLEventProcessorType, "mesh://ecu"), ""},
		// C11 stored the location in place of the id, with any type.
		{"sce://scxml/a%20b", "", ScxmlOrigin("a b"), "a b"},
		// A host's pair has no sender, whether or not its address has a scheme.
		{"motor", "x-sce-host", NewEventOrigin("x-sce-host", "motor"), ""},
		{"mesh://ecu", SCXMLEventProcessorType, NewEventOrigin(SCXMLEventProcessorType, "mesh://ecu"), ""},
	}
	for _, c := range cases {
		origin, sender := ReadStoredOrigin(c.origin, c.originType)
		if !origin.Equal(c.want) || sender != c.sender {
			t.Errorf("ReadStoredOrigin(%q, %q) = (%+v, %q), want (%+v, %q)",
				c.origin, c.originType, origin, sender, c.want, c.sender)
		}
	}
}

func TestAStoredWaitingSendNamesItsSenderByIDOrLocation(t *testing.T) {
	for stored, want := range map[string]string{"s1": "s1", "sce://scxml/a%20b": "a b", "": ""} {
		if got := ReadStoredSender(stored); got != want {
			t.Errorf("ReadStoredSender(%q) = %q, want %q", stored, got, want)
		}
	}
}

