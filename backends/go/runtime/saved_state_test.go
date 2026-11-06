// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import (
	"os"
	"path/filepath"
	"regexp"
	"slices"
	"strings"
	"testing"
)

// The shared saved-state texts every backend reads (docs/adr/0045, section 8):
// format 3 in `saved/` and the unchanged format 1 in `saved/format1/`. The
// format-2 texts in `saved/format2/` are read as other texts, which
// sharedFormat2Texts gives.
func sharedSavedTexts(t *testing.T) map[string]string {
	t.Helper()
	root := filepath.Join("..", "..", "..", "sce-build", "tests", "fixtures", "static_datamodel", "saved")
	texts := map[string]string{}
	for _, directory := range []string{root, filepath.Join(root, "format1")} {
		entries, err := os.ReadDir(directory)
		if err != nil {
			t.Fatalf("the shared saved states: %v", err)
		}
		for _, entry := range entries {
			if entry.IsDir() || filepath.Ext(entry.Name()) != ".json" {
				continue
			}
			path := filepath.Join(directory, entry.Name())
			raw, err := os.ReadFile(path)
			if err != nil {
				t.Fatalf("%s: %v", path, err)
			}
			texts[path] = strings.TrimSpace(string(raw))
		}
	}
	if len(texts) < 20 {
		t.Fatalf("the shared saved states lost instances: %d read", len(texts))
	}
	return texts
}

func TestEverySharedSavedTextReadsAndWritesBackAsItWas(t *testing.T) {
	for path, text := range sharedSavedTexts(t) {
		saved, err := SavedStateFromJSON(text)
		if err != nil {
			t.Errorf("%s: refused: %v", path, err)
			continue
		}
		if written := saved.ToJSON(); written != text {
			t.Errorf("%s: written back as\n%s\nthe text is\n%s", path, written, text)
		}
	}
}

func TestAFormatOneTextIsOfFormatOneAndHoldsNoSession(t *testing.T) {
	text := sharedSavedTexts(t)[filepath.Join("..", "..", "..", "sce-build", "tests", "fixtures",
		"static_datamodel", "saved", "format1", "static_invoke_working.json")]
	saved, err := SavedStateFromJSON(text)
	if err != nil {
		t.Fatal(err)
	}
	if saved.Format() != SavedFormat1 || len(saved.Invokes.IDs) == 0 || saved.SessionID != "" {
		t.Fatalf("format %d, invokes %v, session %q", saved.Format(), saved.Invokes.IDs, saved.SessionID)
	}
	if id, bound := saved.BoundSessionID(); bound {
		t.Fatalf("a format-1 state binds no session id, and binds %q", id)
	}
}

func TestATreeTextHoldsEveryChildWithItsSession(t *testing.T) {
	text := sharedSavedTexts(t)[filepath.Join("..", "..", "..", "sce-build", "tests", "fixtures",
		"static_datamodel", "saved", "static_invoke_tree_working.json")]
	saved, err := SavedStateFromJSON(text)
	if err != nil {
		t.Fatal(err)
	}
	if saved.Format() != SavedFormat3 || len(saved.Invokes.Children) == 0 {
		t.Fatalf("format %d with %d children", saved.Format(), len(saved.Invokes.Children))
	}
	child := saved.Invokes.Children[0]
	if child.Session == nil || child.Session.SessionID != saved.SessionID+"."+child.InvokeID {
		t.Fatalf("the child's session is %+v under %q", child.Session, saved.SessionID)
	}
	if saved.InvokeStart(child.ID) != child.Start {
		t.Fatalf("the parent counts %d starts of '%s', and the child is start %d",
			saved.InvokeStart(child.ID), child.ID, child.Start)
	}
}

const minimalSession = `"shape":"s","sessionid":"root","configuration":["a"],"current":"a","variables":{},` +
	`"history":{},"pending":[],"invokes":[],"invokestarts":{},"hostinvokes":[],"hostinvoketoken":"0",` +
	`"sendseq":"0","external":[]`

func TestATextTheFormatsDoNotCarryIsRefusedForTheMemberThatBreaksIt(t *testing.T) {
	cases := []struct{ name, text, reason string }{
		{"not JSON", `{"format":`, "the saved state is not JSON (it ends where a value was expected)"},
		{"not an object", `[]`, "the saved state is not an object"},
		{"no format", `{}`, "the saved state has no 'format'"},
		{"a format of the future", `{"format":4}`,
			"the saved state is format 4, and this runtime reads formats 1, 2 and 3"},
		{"format as a text", `{"format":"3"}`,
			`the saved state is format "3", and this runtime reads formats 1, 2 and 3`},
		{"a session id that is no Nmtoken",
			`{"format":3,` + strings.Replace(minimalSession, `"root"`, `"a b"`, 1) + `}`,
			"'sessionid' (a b) is not an XML Nmtoken, which a session id is"},
		{"a session id that is no Nmtoken in format 2",
			`{"format":2,` + strings.Replace(minimalSession, `"root"`, `"a b"`, 1) + `}`,
			"'sessionid' (a b) is not an XML Nmtoken, which a session id is"},
		{"a count with a leading zero",
			`{"format":3,` + strings.Replace(minimalSession, `"invokestarts":{}`, `"invokestarts":{"w":"01"}`, 1) + `}`,
			"'invokestarts.w' (01) is not a count of starts, a whole number from 1"},
		{"a token that is no digits",
			`{"format":3,` + strings.Replace(minimalSession, `"hostinvoketoken":"0"`, `"hostinvoketoken":"x"`, 1) + `}`,
			"'hostinvoketoken' (x) is not a whole number a token can be"},
		{"revisions that name nothing",
			`{"format":3,` + minimalSession + `,"revisions":{}}`,
			"'revisions' names no schema; a state whose record schemas are all in their first revision leaves it out"},
		{"a send to the parent in format 1",
			`{"format":1,"shape":"s","configuration":["a"],"current":"a","variables":{},"history":{},` +
				`"pending":[{"due":"5","act":"parent","event":"e","data":"","sendid":"","origin":""}],"invokes":[],` +
				`"hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[]}`,
			"'pending[0].act' is 'parent', which is not raise, internal or host, the acts of format 1"},
		{"an act no format names",
			`{"format":3,` + strings.Replace(minimalSession, `"pending":[]`,
				`"pending":[{"due":"5","act":"fax"}]`, 1) + `}`,
			"'pending[0].act' is 'fax', which is not raise, internal, host, parent or child"},
	}
	for _, c := range cases {
		_, err := SavedStateFromJSON(c.text)
		if err == nil {
			t.Errorf("%s: read", c.name)
			continue
		}
		if err.Error() != c.reason {
			t.Errorf("%s: refused as\n  %s\nwant\n  %s", c.name, err.Error(), c.reason)
		}
	}
}

func TestAChildSessionIsRefusedByItsPath(t *testing.T) {
	withSession := func(session string) string {
		return `{"id":"w","start":"1","invokeid":"w","document":"w","session":{` + session + `}}`
	}
	holding := func(child string) string {
		return strings.Replace(minimalSession, `"invokes":[]`, `"invokes":[`+child+`]`, 1)
	}
	bad := withSession(strings.Replace(minimalSession, `"configuration":["a"]`, `"configuration":[1]`, 1))
	_, err := SavedStateFromJSON(`{"format":3,` + holding(bad) + `}`)
	want := "the session at invokes[0].session: 'configuration' is not a text"
	if err == nil || err.Error() != want {
		t.Fatalf("refused as %v, want %q", err, want)
	}
	_, err = SavedStateFromJSON(`{"format":3,` + holding(withSession(holding(bad))) + `}`)
	want = "the session at invokes[0].session.invokes[0].session: 'configuration' is not a text"
	if err == nil || err.Error() != want {
		t.Fatalf("a grandchild is refused as %v, want %q", err, want)
	}
}

func TestAChildThatStatesAFormatIsRefused(t *testing.T) {
	child := `{"id":"w","start":"1","invokeid":"w","document":"w","session":{"format":3,` + minimalSession + `}}`
	text := `{"format":3,` + strings.Replace(minimalSession, `"invokes":[]`, `"invokes":[`+child+`]`, 1) + `}`
	_, err := SavedStateFromJSON(text)
	want := "'invokes[0].session' states a format, which only the root of a tree does"
	if err == nil || err.Error() != want {
		t.Fatalf("refused as %v, want %q", err, want)
	}
}

// scxmlProcessorMember is the `processor` member of the SCXML processor's pair.
const scxmlProcessorMember = `"processor":"http://www.w3.org/TR/scxml/#SCXMLEventProcessor"`

// queuedEvent is an element of `external` named name, with origin as its
// `origin` and `sender` members.
func queuedEvent(name, origin string) string {
	return `{"name":"` + name + `","data":"","type":"external","sendid":"",` + origin +
		`,"invokeid":"","hostinvoketoken":null}`
}

// withQueued is a format-3 session whose queue holds events and whose
// `pending` holds sends, each list written as its members.
func withQueued(events, sends string) string {
	session := strings.Replace(minimalSession, `"external":[]`, `"external":[`+events+`]`, 1)
	return `{"format":3,` + strings.Replace(session, `"pending":[]`, `"pending":[`+sends+`]`, 1) + `}`
}

// waitingSend is a waiting send of act, with origin as its `origin` and
// `sender` members; a `child` act is for the child `w`.
func waitingSend(act, origin string) string {
	invokeID := ""
	if act == "child" {
		invokeID = `"invokeid":"w",`
	}
	return `{"due":"5","act":"` + act + `",` + invokeID + `"event":"e","data":"","sendid":"s",` + origin + `}`
}

// docs/adr/0090, section 7: a format-3 reader refuses, naming the record, an
// origin that is neither null nor a pair, a pair with a member missing, empty
// or added, a sender that is not a session id, a sender beside a null origin, a
// session's act with no sender or beside an origin other than its SCXML pair,
// and an internal act that holds either member.
func TestAFormatThreeOriginIsRefusedForTheMemberThatBreaksIt(t *testing.T) {
	rootPair := `{` + scxmlProcessorMember + `,"address":"sce://scxml/root"}`
	meshPair := `{` + scxmlProcessorMember + `,"address":"mesh://ecu"}`
	queued := func(origin string) string { return withQueued(queuedEvent("e", origin), "") }
	waiting := func(act, origin string) string { return withQueued("", waitingSend(act, origin)) }
	const notTheActsPair = "'pending[0].origin' is not the SCXML pair of its sender 'root', which is the origin " +
		"the SCXML processor writes for a send of this session"
	const internalHolds = "'pending[0]' is an internal act, whose event is an internal event and has no origin, " +
		"and it holds an origin or a sender"
	cases := []struct{ name, text, reason string }{
		{"an origin that is a text", queued(`"origin":"root","sender":"root"`),
			"'external[0].origin' is neither null nor a pair of 'processor' and 'address'"},
		{"an origin that is a list", queued(`"origin":[],"sender":null`),
			"'external[0].origin' is neither null nor a pair of 'processor' and 'address'"},
		{"a pair with no member", queued(`"origin":{},"sender":null`),
			"'external[0].origin' has no 'processor'"},
		{"a pair with no address", queued(`"origin":{` + scxmlProcessorMember + `},"sender":null`),
			"'external[0].origin' has no 'address'"},
		{"a pair with an empty processor", queued(`"origin":{"processor":"","address":"motor"},"sender":null`),
			"'external[0].origin.processor' is empty, and a pair is whole or null"},
		{"a pair with an empty address", queued(`"origin":{"processor":"x-sce-host","address":""},"sender":null`),
			"'external[0].origin.address' is empty, and a pair is whole or null"},
		{"a pair with a member added",
			queued(`"origin":{"processor":"x-sce-host","address":"motor","sender":"root"},"sender":null`),
			"'external[0].origin' holds 'sender', and a pair holds 'processor' and 'address' alone"},
		{"a pair whose processor is not a text", queued(`"origin":{"processor":1,"address":"motor"},"sender":null`),
			"'external[0].origin.processor' is not a text"},
		{"an event with no origin", queued(`"sender":null`), "'external[0]' has no 'origin'"},
		{"an event with no sender", queued(`"origin":null`), "'external[0]' has no 'sender'"},
		{"an event with the origin of format 2", withQueued(`{"name":"e","data":"","type":"external","sendid":"",`+
			`"origin":"","origintype":"","invokeid":"","hostinvoketoken":null}`, ""),
			"'external[0].origin' is neither null nor a pair of 'processor' and 'address'"},
		{"a sender that is no session id", queued(`"origin":null,"sender":"a b"`),
			"'external[0].sender' (a b) is not an XML Nmtoken, which a session id is"},
		{"a sender that is not a text", queued(`"origin":null,"sender":1`),
			"'external[0].sender' is neither a text nor null"},
		{"a sender beside no origin", queued(`"origin":null,"sender":"root"`),
			"'external[0].origin' is null beside the sender 'root', and a session that sends an event always " +
				"has a reply address"},
		{"a raise act with no sender", waiting("raise", `"origin":null,"sender":null`),
			"'pending[0]' is a raise act, a send of this session, and names no sender"},
		{"a parent act with an origin and no sender", waiting("parent", `"origin":`+meshPair+`,"sender":null`),
			"'pending[0]' is a parent act, a send of this session, and names no sender"},
		{"a child act with no sender", waiting("child", `"origin":null,"sender":null`),
			"'pending[0]' is a child act, a send of this session, and names no sender"},
		{"a waiting send with no sender", waiting("raise", `"origin":`+rootPair),
			"'pending[0]' has no 'sender'"},
		{"a waiting send with the origin of format 2", waiting("raise", `"origin":"root"`),
			"'pending[0].origin' is neither null nor a pair of 'processor' and 'address'"},
		{"a waiting send whose sender is beside another address", waiting("raise", `"origin":`+meshPair+`,"sender":"root"`),
			notTheActsPair},
		{"a waiting send whose sender is beside another processor",
			waiting("child", `"origin":{"processor":"x-sce-host","address":"sce://scxml/root"},"sender":"root"`),
			notTheActsPair},
		{"a waiting send whose sender is beside no origin", waiting("parent", `"origin":null,"sender":"root"`),
			"'pending[0].origin' is null beside the sender 'root', and a session that sends an event always " +
				"has a reply address"},
		{"an internal act that holds a sender", waiting("internal", `"origin":`+rootPair+`,"sender":"root"`),
			internalHolds},
		{"an internal act that holds an origin", waiting("internal", `"origin":`+meshPair+`,"sender":null`),
			internalHolds},
	}
	for _, c := range cases {
		_, err := SavedStateFromJSON(c.text)
		if err == nil {
			t.Errorf("%s: read", c.name)
			continue
		}
		if err.Error() != c.reason {
			t.Errorf("%s: refused as\n  %s\nwant\n  %s", c.name, err.Error(), c.reason)
		}
	}
}

// docs/adr/0090, section 7: a format-3 `raise`, `parent` or `child` act is a
// send of the session that holds it, so one that names another session as its
// sender is refused, in the root and in every child session, and the refusal
// names the session it is about.
func TestAFormatThreeSendOfAnotherSessionIsRefused(t *testing.T) {
	pairOf := func(id string) string {
		return `"origin":{` + scxmlProcessorMember + `,"address":"sce://scxml/` + id + `"},"sender":"` + id + `"`
	}
	_, err := SavedStateFromJSON(withQueued("", waitingSend("raise", pairOf("other"))))
	want := "'pending[0].sender' is 'other', and this session is 'root': a raise act is this session's own send"
	if err == nil || err.Error() != want {
		t.Errorf("the root's act is refused as %v, want %q", err, want)
	}

	child := strings.Replace(strings.Replace(minimalSession, `"sessionid":"root"`, `"sessionid":"root.w"`, 1),
		`"pending":[]`, `"pending":[`+waitingSend("parent", pairOf("root"))+`]`, 1)
	tree := `{"format":3,` + strings.Replace(minimalSession, `"invokes":[]`,
		`"invokes":[{"id":"w","start":"1","invokeid":"w","document":"w","session":{`+child+`}}]`, 1) + `}`
	_, err = SavedStateFromJSON(tree)
	want = "the session at invokes[0].session: 'pending[0].sender' is 'root', and this session is 'root.w': " +
		"a parent act is this session's own send"
	if err == nil || err.Error() != want {
		t.Errorf("the child's act is refused as %v, want %q", err, want)
	}

	// The same tree, each act sent by the session that holds it, is read.
	own := strings.Replace(tree, `"parent","event":"e","data":"","sendid":"s",`+pairOf("root"),
		`"parent","event":"e","data":"","sendid":"s",`+pairOf("root.w"), 1)
	if own == tree {
		t.Fatal("the child's act was not changed")
	}
	if _, err := SavedStateFromJSON(own); err != nil {
		t.Errorf("a tree whose acts are each its session's own is refused: %v", err)
	}
}

// docs/adr/0090, section 7: a format-3 reader takes both members as they are
// written and derives neither from the other, and the text it read is the text
// it writes. A queued event's sender is not tied to its address: the Mesh
// transport delivers a remote child session's event with that session's
// endpoint as the address and its id as the sender.
func TestAFormatThreeOriginIsReadAsItIsWrittenAndWrittenBack(t *testing.T) {
	events := strings.Join([]string{
		queuedEvent("sent", `"origin":{`+scxmlProcessorMember+`,"address":"sce://scxml/peer%3A1"},"sender":"peer:1"`),
		queuedEvent("hosted", `"origin":{"processor":"x-sce-host","address":"motor"},"sender":null`),
		queuedEvent("meshed", `"origin":{`+scxmlProcessorMember+`,"address":"mesh://ecu"},"sender":null`),
		queuedEvent("posted", `"origin":null,"sender":null`),
		queuedEvent("remote", `"origin":{`+scxmlProcessorMember+`,"address":"mesh://dev/machine/session"},`+
			`"sender":"root.w"`),
	}, ",")
	sends := strings.Join([]string{
		waitingSend("raise", `"origin":{`+scxmlProcessorMember+`,"address":"sce://scxml/root"},"sender":"root"`),
		waitingSend("internal", `"origin":null,"sender":null`),
	}, ",")
	text := withQueued(events, sends)
	saved, err := SavedStateFromJSON(text)
	if err != nil {
		t.Fatal(err)
	}
	for at, want := range []struct {
		origin *EventOrigin
		sender string
	}{
		{ScxmlOrigin("peer:1"), "peer:1"},
		{NewEventOrigin("x-sce-host", "motor"), ""},
		{NewEventOrigin(SCXMLEventProcessorType, "mesh://ecu"), ""},
		{nil, ""},
		{NewEventOrigin(SCXMLEventProcessorType, "mesh://dev/machine/session"), "root.w"},
	} {
		event := saved.External[at]
		if !event.Origin.Equal(want.origin) || event.Sender != want.sender {
			t.Errorf("external[%d] is read as (%+v, %q), want (%+v, %q)",
				at, event.Origin, event.Sender, want.origin, want.sender)
		}
	}
	if raise := saved.Pending[0].Act; !raise.Origin.Equal(ScxmlOrigin("root")) || raise.Sender != "root" {
		t.Errorf("the raise act is read as (%+v, %q)", raise.Origin, raise.Sender)
	}
	if internal := saved.Pending[1].Act; internal.Origin != nil || internal.Sender != "" {
		t.Errorf("the internal act is read as (%+v, %q)", internal.Origin, internal.Sender)
	}
	if written := saved.ToJSON(); written != text {
		t.Errorf("written back as\n%s\nthe text is\n%s", written, text)
	}
}

// docs/adr/0090, section 7, "What a format-3 build writes": a format-1 text
// read and written back without a machine stays format 1, its origin read by
// the table and written by the format-2 rule: the sender's id for a session's
// send, the address otherwise, then the pair's processor, both empty for none.
// An internal act's stored origin is not read.
func TestAFormatOneTextIsWrittenBackInFormatOne(t *testing.T) {
	session := func(events, sends string) string {
		return `{"format":1,"shape":"s","configuration":["a"],"current":"a","variables":{},"history":{},` +
			`"pending":[` + sends + `],"invokes":[],"hostinvokes":[],"hostinvoketoken":"0","sendseq":"0",` +
			`"external":[` + events + `]}`
	}
	stored := func(name, origin, originType string) string {
		return queuedEvent(name, `"origin":"`+origin+`","origintype":"`+originType+`"`)
	}
	send := func(act, origin string) string {
		return `{"due":"5","act":"` + act + `","event":"e","data":"","sendid":"s","origin":"` + origin + `"}`
	}
	text := session(strings.Join([]string{
		stored("sent", "peer", SCXMLEventProcessorType),
		stored("hosted", "motor", "x-sce-host"),
		stored("posted", "", ""),
	}, ","), strings.Join([]string{send("raise", "root"), send("raise", "")}, ","))
	saved, err := SavedStateFromJSON(text)
	if err != nil {
		t.Fatal(err)
	}
	if sent := saved.External[0]; !sent.Origin.Equal(ScxmlOrigin("peer")) || sent.Sender != "peer" {
		t.Errorf("a session's id with the SCXML type is read as (%+v, %q)", sent.Origin, sent.Sender)
	}
	if raise := saved.Pending[0].Act; !raise.Origin.Equal(ScxmlOrigin("root")) || raise.Sender != "root" {
		t.Errorf("a raise act's stored id is read as (%+v, %q)", raise.Origin, raise.Sender)
	}
	if written := saved.ToJSON(); written != text {
		t.Errorf("written back as\n%s\nthe text is\n%s", written, text)
	}

	internal, err := SavedStateFromJSON(session("", send("internal", "root")))
	if err != nil {
		t.Fatal(err)
	}
	if act := internal.Pending[0].Act; act.Origin != nil || act.Sender != "" {
		t.Errorf("an internal act's stored origin is read as (%+v, %q), and it is not read", act.Origin, act.Sender)
	}
	if written, want := internal.ToJSON(), session("", send("internal", "")); written != want {
		t.Errorf("an internal act is written back as\n%s\nwant\n%s", written, want)
	}
}

// docs/adr/0090, section 7: a raise, parent or child act of a format-2 text
// whose stored `origin` is empty is read as sent by the session that holds it,
// the `sessionid` of its own session record, so the state is written back as a
// format-3 text that is read. An internal act is not: it has no origin.
func TestAFormatTwoSendWithNoStoredOriginIsASendOfTheSessionHoldingIt(t *testing.T) {
	stored := func(act string) string {
		return `{"due":"5","act":"` + act + `","event":"e","data":"","sendid":"s","origin":""}`
	}
	waiting := func(session string, sends ...string) string {
		return strings.Replace(session, `"pending":[]`, `"pending":[`+strings.Join(sends, ",")+`]`, 1)
	}
	child := waiting(strings.Replace(minimalSession, `"sessionid":"root"`, `"sessionid":"root.w"`, 1),
		stored("raise"), stored("parent"))
	root := strings.Replace(waiting(minimalSession, stored("raise"), stored("internal")), `"invokes":[]`,
		`"invokes":[{"id":"w","start":"1","invokeid":"w","document":"w","session":{`+child+`}}]`, 1)
	saved, err := SavedStateFromJSON(`{"format":2,` + root + `}`)
	if err != nil {
		t.Fatal(err)
	}
	sentBy := func(where string, act SavedAct, id string) {
		t.Helper()
		if !act.Origin.Equal(ScxmlOrigin(id)) || act.Sender != id {
			t.Errorf("%s is read as (%+v, %q), want the pair and the id of '%s'", where, act.Origin, act.Sender, id)
		}
	}
	sentBy("the root's raise act", saved.Pending[0].Act, "root")
	if internal := saved.Pending[1].Act; internal.Origin != nil || internal.Sender != "" {
		t.Errorf("the internal act is read as (%+v, %q)", internal.Origin, internal.Sender)
	}
	resumed := saved.Invokes.Children[0].Session
	sentBy("the child's raise act", resumed.Pending[0].Act, "root.w")
	sentBy("the child's parent act", resumed.Pending[1].Act, "root.w")
	if _, err := SavedStateFromJSON(saved.ToJSON()); err != nil {
		t.Errorf("written back as a text that is refused: %v\n%s", err, saved.ToJSON())
	}
}

// docs/adr/0090, section 7: a format-1 or format-2 `raise`, `parent` or `child`
// act's stored origin names its session by an id or by a location with an id,
// and the sender read from it is an XML Nmtoken. Any other text names no
// session, and the act is refused: the SCXML location's prefix alone, an
// address with no SCXML location, and a location whose id decodes to no
// Nmtoken. A queued event that stored the prefix alone is read as the pair as
// saved, with no sender, and written so.
func TestAnActOriginThatNamesNoSessionIsRefused(t *testing.T) {
	stored := func(kind, origin string) string {
		return `{"due":"5","act":"` + kind + `","event":"e","data":"","sendid":"s","origin":"` + origin + `"}`
	}
	act := func(kind, origin string) string {
		text := `{"format":2,` + strings.Replace(minimalSession, `"pending":[]`,
			`"pending":[`+stored(kind, origin)+`]`, 1) + `}`
		if kind == "child" {
			text = strings.Replace(text, `"act":"child",`, `"act":"child","invokeid":"w",`, 1)
		}
		return text
	}
	refused := func(text, kind, origin string) {
		t.Helper()
		_, err := SavedStateFromJSON(text)
		want := "'pending[0].origin' is '" + origin + "', which names no session: a " + kind + " act is a " +
			"session's own send, and its stored origin is that session's id or its SCXML location"
		if err == nil || err.Error() != want {
			t.Errorf("a %s act of '%s' is refused as %v, want %q", kind, origin, err, want)
		}
	}
	for _, origin := range []string{"sce://scxml/", "mesh://elsewhere", "sce://scxml/a%20b"} {
		for _, kind := range []string{"raise", "parent", "child"} {
			refused(act(kind, origin), kind, origin)
		}
		// Format 1, which has no parent or child act.
		refused(`{"format":1,"shape":"s","configuration":["a"],"current":"a","variables":{},"history":{},`+
			`"pending":[`+stored("raise", origin)+`],"invokes":[],"hostinvokes":[],"hostinvoketoken":"0",`+
			`"sendseq":"0","external":[]}`, "raise", origin)
		if _, err := SavedStateFromJSON(act("internal", origin)); err != nil {
			t.Errorf("an internal act's stored origin '%s' is not read, and it is refused: %v", origin, err)
		}
	}
	// A neighbour that names a session: its id, read from the location.
	for _, origin := range []string{"peer:1", "sce://scxml/peer%3A1"} {
		saved, err := SavedStateFromJSON(act("raise", origin))
		if err != nil {
			t.Fatalf("a raise act of '%s' is refused: %v", origin, err)
		}
		if raise := saved.Pending[0].Act; !raise.Origin.Equal(ScxmlOrigin("peer:1")) || raise.Sender != "peer:1" {
			t.Errorf("a raise act of '%s' is read as (%+v, %q)", origin, raise.Origin, raise.Sender)
		}
	}

	queued := strings.Replace(minimalSession, `"external":[]`, `"external":[`+
		queuedEvent("go", `"origin":"sce://scxml/","origintype":""`)+`]`, 1)
	saved, err := SavedStateFromJSON(`{"format":2,` + queued + `}`)
	if err != nil {
		t.Fatal(err)
	}
	event := saved.External[0]
	if !event.Origin.Equal(NewEventOrigin(SCXMLEventProcessorType, "sce://scxml/")) || event.Sender != "" {
		t.Errorf("the queued event is read as (%+v, %q)", event.Origin, event.Sender)
	}
	if want := `"origin":{` + scxmlProcessorMember + `,"address":"sce://scxml/"},"sender":null`; !strings.Contains(
		saved.ToJSON(), want) {
		t.Errorf("written as %s, want it to hold %s", saved.ToJSON(), want)
	}
}

// sharedFormat2Root is the directory of the format-2 reading fixtures
// (docs/adr/0090, section 7, "The fixtures").
var sharedFormat2Root = filepath.Join("..", "..", "..", "sce-build", "tests", "fixtures", "static_datamodel", "saved",
	"format2")

// sharedFormat2Texts is each format-2 reading fixture by its name, with the
// format-3 text `read_as/` holds for it under the same name.
func sharedFormat2Texts(t *testing.T) map[string][2]string {
	t.Helper()
	entries, err := os.ReadDir(sharedFormat2Root)
	if err != nil {
		t.Fatalf("the format-2 reading fixtures: %v", err)
	}
	texts := map[string][2]string{}
	for _, entry := range entries {
		if entry.IsDir() || filepath.Ext(entry.Name()) != ".json" {
			continue
		}
		var pair [2]string
		for at, path := range []string{
			filepath.Join(sharedFormat2Root, entry.Name()),
			filepath.Join(sharedFormat2Root, "read_as", entry.Name()),
		} {
			raw, err := os.ReadFile(path)
			if err != nil {
				t.Fatalf("%s: %v", path, err)
			}
			pair[at] = strings.TrimSpace(string(raw))
		}
		texts[entry.Name()] = pair
	}
	if len(texts) < 4 {
		t.Fatalf("the format-2 reading fixtures lost instances: %d read", len(texts))
	}
	return texts
}

// docs/adr/0090, section 7: a format-2 text is read by the table of section 7,
// each queued event's origin and each waiting send's as the pair and the sender
// the table gives, and is written back as format 3: the text `read_as/` holds.
// That text is format 3, and reads and writes back as itself.
func TestAFormatTwoTextIsReadByTheTableAndWrittenBackAsFormatThree(t *testing.T) {
	for name, texts := range sharedFormat2Texts(t) {
		format2, readAs := texts[0], texts[1]
		saved, err := SavedStateFromJSON(format2)
		if err != nil {
			t.Errorf("%s: refused: %v", name, err)
			continue
		}
		if saved.Format() != SavedFormat3 {
			t.Errorf("%s: a tree read from format 2 is written as format %d", name, saved.Format())
		}
		if written := saved.ToJSON(); written != readAs {
			t.Errorf("%s: written back as\n%s\nread_as/ holds\n%s", name, written, readAs)
		}
		again, err := SavedStateFromJSON(readAs)
		if err != nil {
			t.Errorf("read_as/%s: refused: %v", name, err)
			continue
		}
		if written := again.ToJSON(); written != readAs {
			t.Errorf("read_as/%s: written back as\n%s\nthe text is\n%s", name, written, readAs)
		}
	}
}

func TestAJSONNumberKeepsTheSpellingItWasWrittenIn(t *testing.T) {
	text := `{"a":[1,-0,1.50,1E+2,9223372036854775808],"b":null,"c":"\u00e9\ud83d\ude00"}`
	value, err := ParseJSON(text)
	if err != nil {
		t.Fatal(err)
	}
	// A character written as a \u escape is written as the character.
	want := `{"a":[1,-0,1.50,1E+2,9223372036854775808],"b":null,"c":"é😀"}`
	if got := WriteJSON(value); got != want {
		t.Fatalf("written as %s, want %s", got, want)
	}
}

func TestAJSONObjectKeepsTheOrderItsMembersWereWrittenIn(t *testing.T) {
	value, err := ParseJSON(`{"z":1,"a":2,"m":3}`)
	if err != nil {
		t.Fatal(err)
	}
	if got := strings.Join(value.(*JSONObject).Keys(), ""); got != "zam" {
		t.Fatalf("the members are %s", got)
	}
}

func TestATextThatIsNotOneJSONValueIsRefused(t *testing.T) {
	cases := map[string]string{
		`{"a":1,"a":2}`: `not JSON (the field "a" appears twice)`,
		`1 2`:           "more than one JSON value",
		`[1,]`:          "not JSON (unexpected ']')",
		`"\q"`:          `not JSON (unknown escape '\q')`,
		`"\u12"`:        `not JSON (a \u escape is short)`,
		`"\u12zz"`:      `not JSON (a \u escape is not hex)`,
		`-`:             "not JSON (a number has no digits)",
		`{"a" 1}`:       "not JSON (a ':' was expected)",
		`tru`:           "not JSON (expected 'true')",
	}
	for text, want := range cases {
		if _, err := ParseJSON(text); err == nil || err.Error() != want {
			t.Errorf("%s: refused as %v, want %q", text, err, want)
		}
	}
}

func TestALoneSurrogateReadsAsTheReplacementCharacter(t *testing.T) {
	value, err := ParseJSON(`"\ud800x\udc00"`)
	if err != nil {
		t.Fatal(err)
	}
	if got := value.(string); got != "\uFFFDx\uFFFD" {
		t.Fatalf("read as %q", got)
	}
}

func TestEveryValueTypeIsWrittenAsItsSavedFormAndReadBackAsItWas(t *testing.T) {
	if got := WriteJSON(SavedWriteUint8(200)); got != "200" {
		t.Errorf("uint8 is written as %s", got)
	}
	if got := WriteJSON(SavedWriteInt64(-9223372036854775808)); got != `"-9223372036854775808"` {
		t.Errorf("a 64-bit integer is written as %s", got)
	}
	if got := WriteJSON(SavedWriteUint64(18446744073709551615)); got != `"18446744073709551615"` {
		t.Errorf("a 64-bit unsigned integer is written as %s", got)
	}
	if got := WriteJSON(SavedWriteFloat32(0.3)); got != "0.30000001192092896" {
		t.Errorf("a 32-bit real is written as %s", got)
	}
	if got := WriteJSON(SavedWriteFloat64(5)); got != "5" {
		t.Errorf("a whole real is written as %s", got)
	}
	if got := WriteJSON(SavedWriteFloat64(1e21)); got != "1e+21" {
		t.Errorf("a large real is written as %s", got)
	}
	if got := WriteJSON(SavedWriteBytes([]byte{0, 0xE9, 0xFF})); got != "\"\\u0000\u00e9\u00ff\"" {
		t.Errorf("a byte string is written as %s", got)
	}
	cases := []struct {
		name string
		text string
		read func(any) error
	}{
		{"uint8", `255`, func(v any) error { _, err := SavedReadUint8(v, "x"); return err }},
		{"int8", `-128`, func(v any) error { _, err := SavedReadInt8(v, "x"); return err }},
		{"uint32", `4294967295`, func(v any) error { _, err := SavedReadUint32(v, "x"); return err }},
		{"int64", `"-9223372036854775808"`, func(v any) error { _, err := SavedReadInt64(v, "x"); return err }},
		{"uint64", `"18446744073709551615"`, func(v any) error { _, err := SavedReadUint64(v, "x"); return err }},
		{"float64", `1.5`, func(v any) error { _, err := SavedReadFloat64(v, "x"); return err }},
		{"a non-finite real", `"-Infinity"`, func(v any) error { _, err := SavedReadFloat32(v, "x"); return err }},
		{"a real past the range", `1e999`, func(v any) error { _, err := SavedReadFloat64(v, "x"); return err }},
		{"bool", `true`, func(v any) error { _, err := SavedReadBool(v, "x"); return err }},
	}
	for _, c := range cases {
		value, err := ParseJSON(c.text)
		if err != nil {
			t.Fatal(err)
		}
		if err := c.read(value); err != nil {
			t.Errorf("%s: %s is refused: %v", c.name, c.text, err)
		}
	}
}

func TestAValueOfAnotherKindOrAnotherWidthIsRefusedNotConverted(t *testing.T) {
	read := func(text string, f func(any) error) string {
		value, err := ParseJSON(text)
		if err != nil {
			t.Fatal(err)
		}
		if err := f(value); err != nil {
			return err.Error()
		}
		return ""
	}
	uint8Of := func(v any) error { _, err := SavedReadUint8(v, "x"); return err }
	int64Of := func(v any) error { _, err := SavedReadInt64(v, "x"); return err }
	stringOf := func(v any) error { _, err := SavedReadString(v, "x"); return err }
	boolOf := func(v any) error { _, err := SavedReadBool(v, "x"); return err }
	for _, c := range []struct {
		text string
		read func(any) error
		want string
	}{
		{`256`, uint8Of, "'x' (256) is not a whole number its type can hold"},
		{`-1`, uint8Of, "'x' (-1) is not a whole number its type can hold"},
		{`1.5`, uint8Of, "'x' (1.5) is not a whole number its type can hold"},
		{`"1"`, uint8Of, "'x' is not a number"},
		{`null`, uint8Of, "'x' is absent (null), and its type is not optional"},
		{`5`, int64Of, "'x' is not a 64-bit whole number written as a text"},
		{`"9223372036854775808"`, int64Of, "'x' (9223372036854775808) is not a whole number its type can hold"},
		{`1`, stringOf, "'x' is not a text"},
		{`"true"`, boolOf, "'x' is not a truth value"},
	} {
		if got := read(c.text, c.read); got != c.want {
			t.Errorf("%s: refused as %q, want %q", c.text, got, c.want)
		}
	}
}

func TestATextAndAListAreHeldToTheBoundsTheMachineHas(t *testing.T) {
	if _, err := SavedReadBoundedString("héllo", "t", 5); err == nil ||
		err.Error() != "'t' holds 6 UTF-8 bytes, past the 5 it is bounded by" {
		t.Errorf("a text past its bound: %v", err)
	}
	if _, err := SavedReadBytes("a\u0100", "b", 8); err == nil ||
		err.Error() != "'b' holds U+0100, which is past U+00FF and so no byte" {
		t.Errorf("a character past U+00FF: %v", err)
	}
	if _, err := SavedReadBytes("abc", "b", 2); err == nil ||
		err.Error() != "'b' holds 3 bytes, past the 2 it is bounded by" {
		t.Errorf("bytes past their bound: %v", err)
	}
	element := func(v any, what string) (uint8, error) { return SavedReadUint8(v, what) }
	items, err := ParseJSON(`[1,2,300]`)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := SavedReadList(items, "l", 8, element); err == nil ||
		err.Error() != "'l[2]' (300) is not a whole number its type can hold" {
		t.Errorf("an element no uint8 holds: %v", err)
	}
	if _, err := SavedReadList(items, "l", 2, element); err == nil ||
		err.Error() != "'l' holds 3 elements, past the 2 it is bounded by" {
		t.Errorf("a list past its bound: %v", err)
	}
}

func TestAnUpgradeGivesARecordTheDefaultsOfTheFieldsALaterRevisionAdded(t *testing.T) {
	digest := strings.Repeat("a", 64)
	shape := &SavedShape{
		First: "first", Revised: "revised", Format2First: "first2", Format2Revised: "revised2",
		Records: []RecordLineage{{
			Event: "shown", Revision: 2, Revisions: []string{digest},
			Added:   []AddedField{{ID: "weekday", Since: 2, Default: "0"}},
			Holders: []RecordHolder{{Variable: "one", List: false}, {Variable: "many", List: true}},
		}},
	}
	variables, err := ParseJSON(`{"one":{"year":2027},"many":[{"year":1},{"year":2}]}`)
	if err != nil {
		t.Fatal(err)
	}
	saved := &SavedState{Shape: "first2", Variables: variables.(*JSONObject), Invokes: SavedInvokes{Resumed: true}}
	upgraded, err := UpgradeSavedState(saved, shape)
	if err != nil {
		t.Fatal(err)
	}
	want := `{"one":{"year":2027,"weekday":0},"many":[{"year":1,"weekday":0},{"year":2,"weekday":0}]}`
	if got := WriteJSON(upgraded.Variables); got != want {
		t.Fatalf("grown to %s, want %s", got, want)
	}
	if WriteJSON(saved.Variables) != `{"one":{"year":2027},"many":[{"year":1},{"year":2}]}` {
		t.Fatal("the upgrade changed the state it was given")
	}

	// A record that already holds a field its revision does not declare.
	held, _ := ParseJSON(`{"one":{"year":1,"weekday":3},"many":[]}`)
	saved.Variables = held.(*JSONObject)
	if _, err := UpgradeSavedState(saved, shape); err == nil ||
		err.Error() != "'one' holds 'weekday', which revision 1 of 'shown' does not declare: revision 2 added it" {
		t.Fatalf("a field of a revision the state was not saved at: %v", err)
	}

	// A state of another shape.
	saved.Shape = "other"
	if _, err := UpgradeSavedState(saved, shape); err == nil ||
		err.Error() != "the saved state is of a document of shape other, and this machine's is first2" {
		t.Fatalf("a state of another shape: %v", err)
	}

	// A revision later than the machine's.
	saved.Shape = "revised2"
	saved.Revisions = []SavedRevision{{Key: "shown", Revision: 3, Fields: digest}}
	if _, err := UpgradeSavedState(saved, shape); err == nil ||
		err.Error() != "the saved state was saved at revision 3 of 'shown', later than this machine's 2: a machine does not restore what it does not know" {
		t.Fatalf("a later revision: %v", err)
	}
}

// docs/adr/0055, section 6: the third revision of `shown` widened `year`, a field
// of the first, and `weekday`, which the second added, to optional. A record saved
// before the widening holds the field as a value, so a null in it is refused,
// naming the field and the revision; saved at the widening, null is absent.
func TestANullSavedBeforeAFieldWasWidenedIsRefused(t *testing.T) {
	second, third := strings.Repeat("2", 64), strings.Repeat("3", 64)
	shape := &SavedShape{
		First: "first", Revised: "revised", Format2First: "first2", Format2Revised: "revised2",
		Records: []RecordLineage{{
			Event: "shown", Revision: 3, Revisions: []string{second, third},
			Added:   []AddedField{{ID: "weekday", Since: 2, Default: "0"}},
			Widened: []WidenedField{{ID: "year", Since: 1, OptionalSince: 3}, {ID: "weekday", Since: 2, OptionalSince: 3}},
			Holders: []RecordHolder{{Variable: "one", List: false}, {Variable: "many", List: true}},
		}},
	}
	savedAt := func(revision int, one, many string) *SavedState {
		t.Helper()
		variables, err := ParseJSON(`{"one":` + one + `,"many":` + many + `}`)
		if err != nil {
			t.Fatal(err)
		}
		state := &SavedState{Shape: "first2", Variables: variables.(*JSONObject), Invokes: SavedInvokes{Resumed: true}}
		if revision > 1 {
			state.Shape = "revised2"
			digest := second
			if revision == 3 {
				digest = third
			}
			state.Revisions = []SavedRevision{{Key: "shown", Revision: revision, Fields: digest}}
		}
		return state
	}
	refused := func(saved *SavedState, want string) {
		t.Helper()
		if _, err := UpgradeSavedState(saved, shape); err == nil || !strings.Contains(err.Error(), want) {
			t.Fatalf("refused as %v, want %q", err, want)
		}
	}

	// A record variable, a field of the first revision, saved at the first.
	refused(savedAt(1, `{"year":null}`, `[]`),
		"'one.year' is null, which revision 1 of 'shown' holds as a value: revision 3 widened it")
	// An element of a list of records.
	refused(savedAt(1, `{"year":4}`, `[{"year":1},{"year":null}]`), "'many[1].year' is null")
	// A field the second revision added, saved at the second.
	refused(savedAt(2, `{"year":4,"weekday":null}`, `[]`),
		"'one.weekday' is null, which revision 2 of 'shown' holds as a value: revision 3 widened it")
	// The field the second added, held by a record saved at the first, is refused as
	// a field that revision does not declare, which is the truer reason.
	refused(savedAt(1, `{"year":4,"weekday":null}`, `[]`), "'one' holds 'weekday'")

	// Saved at the revision that widened both, null is absent.
	kept := savedAt(3, `{"year":null,"weekday":null}`, `[{"year":null,"weekday":2}]`)
	upgraded, err := UpgradeSavedState(kept, shape)
	if err != nil {
		t.Fatalf("saved at the widening: %v", err)
	}
	if WriteJSON(upgraded.Variables) != WriteJSON(kept.Variables) {
		t.Fatalf("grown to %s, want %s", WriteJSON(upgraded.Variables), WriteJSON(kept.Variables))
	}
	// A record saved before the widening, each field present, restores as it was.
	upgraded, err = UpgradeSavedState(savedAt(1, `{"year":4}`, `[{"year":1}]`), shape)
	if err != nil {
		t.Fatalf("saved before the widening: %v", err)
	}
	if got, want := WriteJSON(upgraded.Variables),
		`{"one":{"year":4,"weekday":0},"many":[{"year":1,"weekday":0}]}`; got != want {
		t.Fatalf("grown to %s, want %s", got, want)
	}
	// What is no record where one is held is left to the variable's own read.
	upgraded, err = UpgradeSavedState(savedAt(1, `null`, `[{}]`), shape)
	if err != nil {
		t.Fatalf("a variable that holds no record: %v", err)
	}
	if got, want := WriteJSON(upgraded.Variables), `{"one":null,"many":[{"weekday":0}]}`; got != want {
		t.Fatalf("grown to %s, want %s", got, want)
	}
}

func TestAnEnumVariantALaterRevisionAddedIsRefusedInAStateSavedBeforeIt(t *testing.T) {
	digest := strings.Repeat("b", 64)
	field := "status"
	shape := &SavedShape{
		Format2First: "first2", Format2Revised: "revised2",
		Enums: []EnumLineage{{
			Key: "enum:status", Revision: 2, Revisions: []string{digest},
			Added:   []AddedVariant{{Name: "Done", Since: 2}},
			Holders: []EnumHolder{{Variable: "item", Field: &field}, {Variable: "state"}},
		}},
	}
	variables, _ := ParseJSON(`{"item":{"status":"Done"},"state":"Open"}`)
	saved := &SavedState{Shape: "first2", Variables: variables.(*JSONObject), Invokes: SavedInvokes{Resumed: true}}
	_, err := UpgradeSavedState(saved, shape)
	want := "'item.status' holds 'Done', which revision 1 of 'enum:status' does not declare: revision 2 added it"
	if err == nil || err.Error() != want {
		t.Fatalf("refused as %v, want %q", err, want)
	}
}

func TestASessionIDIsAnXMLNmtokenAndADrawnOneIsAVersion4UUID(t *testing.T) {
	for _, id := range []string{"fixture-root", "a.b:c_d", "é1", "x\u00b7y"} {
		if reason := SessionIDRefusalOf(id); reason != "" {
			t.Errorf("%q is refused: %s", id, reason)
		}
	}
	cases := map[string]string{
		"": "the session id is empty, and a session id is one name character or more (an XML Nmtoken)",
		"a b": "the session id holds U+0020, which is not a name character: a session id is an XML Nmtoken " +
			"(letters, digits, '.', '-', '_', ':')",
		"\xff": "the session id holds U+FFFD, which is not a name character: a session id is an XML Nmtoken " +
			"(letters, digits, '.', '-', '_', ':')",
	}
	for id, want := range cases {
		if got := SessionIDRefusalOf(id); got != want {
			t.Errorf("%q is refused as %q, want %q", id, got, want)
		}
	}
	pattern := regexp.MustCompile(`^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$`)
	first, second := DrawSessionID(), DrawSessionID()
	if !pattern.MatchString(first) || !IsNmtoken(first) || first == second {
		t.Fatalf("drew %s and %s", first, second)
	}
}

func TestARefusalInsideAChildNamesTheSessionItIsAbout(t *testing.T) {
	inner := RefusalWithin(Refuse("'x' is not a text"), "invokes[1].session")
	if inner.Error() != "the session at invokes[1].session: 'x' is not a text" {
		t.Fatal(inner.Error())
	}
	outer := RefusalWithin(inner, "invokes[0].session")
	if outer.Error() != "the session at invokes[0].session.invokes[1].session: 'x' is not a text" {
		t.Fatal(outer.Error())
	}
}

// featuredSession is a tree's session of shape standing in `a`, holding
// nothing, taken in the configuration features.
func featuredSession(shape string, features []string) *SavedState {
	return &SavedState{
		Shape: shape, Configuration: []string{"a"}, Current: "a", Variables: NewJSONObject(),
		Invokes: SavedInvokes{Resumed: true}, SessionID: "root", Features: features,
	}
}

// The configuration a state was taken in is written as the last member of its
// session, after `revisions`, and read back as it was; a machine of no product
// line writes none, and its text is the one it was before configurations were
// recorded (docs/adr/0057 §7).
func TestASavedStateIsReadBackAsItWasWritten(t *testing.T) {
	state := featuredSession("s", []string{"session-unicast", "tls"})
	state.Revisions = []SavedRevision{{Key: "reading.taken", Revision: 3, Fields: strings.Repeat("0", 64)}}
	text := state.ToJSON()
	if want := `,"revisions":{"reading.taken":{"revision":3,"fields":"` + strings.Repeat("0", 64) +
		`"}},"features":["session-unicast","tls"]}`; !strings.HasSuffix(text, want) {
		t.Fatalf("written as %s, want it to end %s", text, want)
	}
	back, err := SavedStateFromJSON(text)
	if err != nil {
		t.Fatal(err)
	}
	if !slices.Equal(back.Features, state.Features) {
		t.Fatalf("read back with the features %q, want %q", back.Features, state.Features)
	}
	if written := back.ToJSON(); written != text {
		t.Fatalf("written back as\n%s\nthe text is\n%s", written, text)
	}

	plain := *state
	plain.Features = nil
	if strings.Contains(plain.ToJSON(), "features") {
		t.Fatalf("a machine of no product line writes %s", plain.ToJSON())
	}
	back, err = SavedStateFromJSON(plain.ToJSON())
	if err != nil {
		t.Fatal(err)
	}
	if back.Features != nil || back.ToJSON() != plain.ToJSON() {
		t.Fatalf("read back with the features %q as %s", back.Features, back.ToJSON())
	}

	// The configuration that turns no feature on is one, not the absence of one.
	noneOn := *state
	noneOn.Features = []string{}
	if !strings.HasSuffix(noneOn.ToJSON(), `,"features":[]}`) {
		t.Fatalf("the configuration of no feature is written as %s", noneOn.ToJSON())
	}
	back, err = SavedStateFromJSON(noneOn.ToJSON())
	if err != nil {
		t.Fatal(err)
	}
	if back.Features == nil || len(back.Features) != 0 {
		t.Fatalf("the configuration of no feature is read back as %#v", back.Features)
	}
}

// A restore judges the configuration a state was taken in before its shape, and
// names the features that differ (docs/adr/0057 §7).
func TestAStateOfAnotherConfigurationIsRefusedByTheFeaturesThatDiffer(t *testing.T) {
	taken := func(features []string) *SavedState { return featuredSession("first2", features) }
	refused := func(saved *SavedState, built []string, want string) {
		t.Helper()
		err := CheckSavedFeatures(saved, built)
		if err == nil || err.Error() != want {
			t.Errorf("refused as %v, want %q", err, want)
		}
	}
	for _, kept := range []struct{ saved, built []string }{
		{nil, nil}, {[]string{}, []string{}}, {[]string{"a", "b"}, []string{"a", "b"}},
	} {
		if err := CheckSavedFeatures(taken(kept.saved), kept.built); err != nil {
			t.Errorf("%#v against %#v: %v", kept.saved, kept.built, err)
		}
	}
	refused(taken([]string{"a", "b"}), []string{"a", "c"},
		"the saved state was taken with the features [a, b] on, and this machine is built with [a, c]: "+
			"[b] on only where it was saved, [c] on only in this build")
	refused(taken([]string{"a"}), []string{},
		"the saved state was taken with the features [a] on, and this machine is built with []: "+
			"[a] on only where it was saved")
	refused(taken(nil), []string{"a"},
		"the saved state records no features, and this machine is built with the features [a] on")
	refused(taken([]string{}), nil,
		"the saved state was taken with the features [] on, and this machine is built from a document of "+
			"no feature model")
	// No configuration is not the configuration of no feature.
	refused(taken(nil), []string{},
		"the saved state records no features, and this machine is built with the features [] on")

	// Judged before the shape: a state of another build is refused for its
	// features even where its shape differs too.
	shape := &SavedShape{
		First: "first", Revised: "revised", Format2First: "first2", Format2Revised: "revised2",
		Features: []string{"a"},
	}
	other := taken([]string{"b"})
	other.Shape = "other"
	if _, err := UpgradeSavedState(other, shape); err == nil ||
		!strings.HasPrefix(err.Error(), "the saved state was taken with the features [b] on") {
		t.Fatalf("a state of another configuration and shape is refused as %v", err)
	}
	// Taken in the machine's own configuration, it is judged by its shape.
	if _, err := UpgradeSavedState(taken([]string{"a"}), shape); err != nil {
		t.Fatalf("a state of the machine's own configuration: %v", err)
	}
}

// `features` is a set, written sorted: a text that repeats a feature or names
// them out of order is refused as it is read.
func TestTheFeaturesOfASavedStateAreASortedSet(t *testing.T) {
	text := func(features string) string {
		return `{"format":1,"shape":"s","configuration":["s"],"current":"s","variables":{},"history":{},` +
			`"pending":[],"invokes":[],"hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[],` +
			`"features":` + features + `}`
	}
	read, err := SavedStateFromJSON(text(`["a","b"]`))
	if err != nil {
		t.Fatal(err)
	}
	if !slices.Equal(read.Features, []string{"a", "b"}) {
		t.Fatalf("read as %q", read.Features)
	}
	for features, want := range map[string]string{
		`["b","a"]`:    "'features' is not in order or names a feature twice; it is a set, sorted",
		`["a","a"]`:    "'features' is not in order or names a feature twice; it is a set, sorted",
		`"a"`:          "'features' is not an array",
		`{"tls":true}`: "'features' is not an array",
		`["b","a",1]`:  "'features' is not a text",
		`["a",null]`:   "'features' is not a text",
	} {
		if _, err := SavedStateFromJSON(text(features)); err == nil || err.Error() != want {
			t.Errorf("%s: refused as %v, want %q", features, err, want)
		}
	}
}
