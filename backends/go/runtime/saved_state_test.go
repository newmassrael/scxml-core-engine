// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import (
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"testing"
)

// The shared saved-state texts every backend reads (docs/adr/0045, section 8):
// format 2 in `saved/` and the unchanged format 1 in `saved/format1/`.
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
	if saved.Format() != SavedFormat2 || len(saved.Invokes.Children) == 0 {
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
		{"a format of the future", `{"format":3}`,
			"the saved state is format 3, and this runtime reads formats 1 and 2"},
		{"format as a text", `{"format":"2"}`,
			`the saved state is format "2", and this runtime reads formats 1 and 2`},
		{"a session id that is no Nmtoken",
			`{"format":2,` + strings.Replace(minimalSession, `"root"`, `"a b"`, 1) + `}`,
			"'sessionid' (a b) is not an XML Nmtoken, which a session id is"},
		{"a count with a leading zero",
			`{"format":2,` + strings.Replace(minimalSession, `"invokestarts":{}`, `"invokestarts":{"w":"01"}`, 1) + `}`,
			"'invokestarts.w' (01) is not a count of starts, a whole number from 1"},
		{"a token that is no digits",
			`{"format":2,` + strings.Replace(minimalSession, `"hostinvoketoken":"0"`, `"hostinvoketoken":"x"`, 1) + `}`,
			"'hostinvoketoken' (x) is not a whole number a token can be"},
		{"revisions that name nothing",
			`{"format":2,` + minimalSession + `,"revisions":{}}`,
			"'revisions' names no schema; a state whose record schemas are all in their first revision leaves it out"},
		{"a send to the parent in format 1",
			`{"format":1,"shape":"s","configuration":["a"],"current":"a","variables":{},"history":{},` +
				`"pending":[{"due":"5","act":"parent","event":"e","data":"","sendid":"","origin":""}],"invokes":[],` +
				`"hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[]}`,
			"'pending[0].act' is 'parent', which is not raise, internal or host, the acts of format 1"},
		{"an act no format names",
			`{"format":2,` + strings.Replace(minimalSession, `"pending":[]`,
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
	_, err := SavedStateFromJSON(`{"format":2,` + holding(bad) + `}`)
	want := "the session at invokes[0].session: 'configuration' is not a text"
	if err == nil || err.Error() != want {
		t.Fatalf("refused as %v, want %q", err, want)
	}
	_, err = SavedStateFromJSON(`{"format":2,` + holding(withSession(holding(bad))) + `}`)
	want = "the session at invokes[0].session.invokes[0].session: 'configuration' is not a text"
	if err == nil || err.Error() != want {
		t.Fatalf("a grandchild is refused as %v, want %q", err, want)
	}
}

func TestAChildThatStatesAFormatIsRefused(t *testing.T) {
	child := `{"id":"w","start":"1","invokeid":"w","document":"w","session":{"format":2,` + minimalSession + `}}`
	text := `{"format":2,` + strings.Replace(minimalSession, `"invokes":[]`, `"invokes":[`+child+`]`, 1) + `}`
	_, err := SavedStateFromJSON(text)
	want := "'invokes[0].session' states a format, which only the root of a tree does"
	if err == nil || err.Error() != want {
		t.Fatalf("refused as %v, want %q", err, want)
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
