// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// tests/json_text/payload_refusal.json, read here with encoding/json: every
// case read through this runtime's payload reader, the code, pointer and bound
// of each refusal held to what the table says, and each case it accepts read
// (docs/adr/0087, section 9.1, item 3). The table is the one every runtime's
// reader is held to.

package sce

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// refusalCase is one case of the shared table.
type refusalCase struct {
	Name string `json:"name"`
	Data string `json:"data"`
	Type any    `json:"type"`
	// Depth, where a case gives one, has the case read as a `<sce:lift>` of the
	// record of the one field `v` reads it (docs/adr/0087, section 2.1).
	Depth *uint32 `json:"depth"`
	// PathBound holds a lift's tooDeep pointer to its bytes: the standard
	// refusal's 256 where a case gives none.
	PathBound *uint32 `json:"pathBound"`
	Refusal   *struct {
		Code    string  `json:"code"`
		Pointer string  `json:"pointer"`
		Bound   *uint32 `json:"bound"`
	} `json:"refusal"`
}

// readDescribed reads v as the type ty the table describes, one call per
// constructor, as a generated reader reads it.
func readDescribed(ty any, v PayloadValue) error {
	switch t := ty.(type) {
	case string:
		var err error
		switch t {
		case "uint8":
			_, err = ValueUnsigned[uint8](v)
		case "uint16":
			_, err = ValueUnsigned[uint16](v)
		case "uint32":
			_, err = ValueUnsigned[uint32](v)
		case "uint64":
			_, err = ValueUnsigned64(v)
		case "int8":
			_, err = ValueSigned[int8](v)
		case "int16":
			_, err = ValueSigned[int16](v)
		case "int32":
			_, err = ValueSigned[int32](v)
		case "int64":
			_, err = ValueSigned64(v)
		case "float32":
			_, err = ValueFloat[float32](v)
		case "float64":
			_, err = ValueFloat[float64](v)
		case "bool":
			_, err = v.Bool()
		case "string":
			_, err = v.Name()
		default:
			panic("the table names no type " + t)
		}
		return err
	case []any:
		name, err := v.Name()
		if err != nil {
			return err
		}
		for _, declared := range t {
			if declared == name {
				return nil
			}
		}
		return v.UnknownVariant(name, "Described")
	case map[string]any:
		if inner, ok := t["optional"]; ok {
			if v.IsNull() {
				return nil
			}
			return readDescribed(inner, v)
		}
		if bound, ok := t["string"].(float64); ok {
			_, err := v.Text(int(bound))
			return err
		}
		if bound, ok := t["bytes"].(float64); ok {
			_, err := v.Bytes(int(bound))
			return err
		}
		if inner, ok := t["list"]; ok {
			bound, _ := t["max"].(float64)
			// The lower count, where the case states one (docs/adr/0086, section 15).
			least, _ := t["min"].(float64)
			_, err := ReadListWithin(v, int(least), int(bound), func(element PayloadValue) (struct{}, error) {
				return struct{}{}, readDescribed(inner, element)
			})
			return err
		}
		if fields, ok := t["record"].(map[string]any); ok {
			record, err := v.Record("")
			if err != nil {
				return err
			}
			for id, field := range fields {
				value, err := record.Field(id)
				if err != nil {
					return err
				}
				if err := readDescribed(field, value); err != nil {
					return err
				}
			}
			return nil
		}
		if described, ok := t["union"].(map[string]any); ok {
			// The tag's enum is the cases' names; an empty case is null, any
			// other the record it holds (docs/adr/0092, section 4).
			tag, _ := described["tag"].(string)
			cases, _ := described["cases"].(map[string]any)
			_, err := ReadUnion(v, tag, func(named PayloadValue) (string, error) {
				name, err := named.Name()
				if err != nil {
					return "", err
				}
				if _, declared := cases[name]; !declared {
					return "", named.UnknownVariant(name, "Described")
				}
				return name, nil
			}, func(name string, object PayloadValue) (struct{}, error) {
				if record := cases[name]; record != nil {
					return struct{}{}, readDescribed(record, object)
				}
				return struct{}{}, nil
			})
			return err
		}
	}
	panic(fmt.Sprintf("a type of the table names no constructor: %v", ty))
}

func TestEveryCaseOfTheSharedTableIsRefusedWithItsCodePointerAndBound(t *testing.T) {
	raw, err := os.ReadFile(filepath.Join("..", "..", "..", "tests", "json_text", "payload_refusal.json"))
	if err != nil {
		t.Fatalf("the shared refusal table: %v", err)
	}
	var table struct {
		Cases []refusalCase `json:"cases"`
	}
	if err := json.Unmarshal(raw, &table); err != nil {
		t.Fatalf("the table is JSON: %v", err)
	}
	if len(table.Cases) < 40 {
		t.Fatalf("the table lost cases: %d", len(table.Cases))
	}
	for _, c := range table.Cases {
		read := func() error {
			var fields PayloadFields
			var err error
			if c.Depth != nil {
				pathBound := uint32(256)
				if c.PathBound != nil {
					pathBound = *c.PathBound
				}
				fields, err = LiftText(c.Data, *c.Depth, pathBound, "")
			} else {
				fields, err = LiftPayload(c.Data)
			}
			if err != nil {
				return err
			}
			v, err := fields.Field("v")
			if err != nil {
				return err
			}
			return readDescribed(c.Type, v)
		}()
		if c.Refusal == nil {
			if read != nil {
				t.Errorf("%s: refused (%v), the table reads it", c.Name, read)
			}
			continue
		}
		var refusal *PayloadRefusal
		if !errors.As(read, &refusal) {
			t.Errorf("%s: read %v, the table refuses it as %s", c.Name, read, c.Refusal.Code)
			continue
		}
		code, ok := RefusalCodeNamed(c.Refusal.Code)
		if !ok {
			t.Fatalf("%s: the enum declares no %s", c.Name, c.Refusal.Code)
		}
		bound, hasBound := refusal.Bound()
		wantBound := c.Refusal.Bound != nil
		if refusal.Code() != code || refusal.Pointer() != c.Refusal.Pointer || hasBound != wantBound ||
			(wantBound && bound != *c.Refusal.Bound) {
			t.Errorf("%s: refused %s at %q bound %v/%v (%v), the table says %s at %q bound %v",
				c.Name, refusal.Code().Name(), refusal.Pointer(), bound, hasBound, refusal,
				c.Refusal.Code, c.Refusal.Pointer, c.Refusal.Bound)
		}
	}
}

// The codes are the variants sce:std/lift/refusal_code.scxml declares, by the
// same names and at the same numbers.
func TestEveryCodeIsTheVariantTheStandardEnumDeclares(t *testing.T) {
	raw, err := os.ReadFile(filepath.Join("..", "..", "..", "stdlib", "lift", "refusal_code.scxml"))
	if err != nil {
		t.Fatalf("the standard refusal code: %v", err)
	}
	document := string(raw)
	// RefusalTooFew is the last code (docs/adr/0086, section 15.11).
	if declared := strings.Count(document, "<sce:variant "); declared != int(RefusalTooFew) {
		t.Fatalf("the enum declares %d variants, this runtime %d codes", declared, RefusalTooFew)
	}
	for code := RefusalNotJSON; code <= RefusalTooFew; code++ {
		variant := fmt.Sprintf(`<sce:variant name="%s" value="%d"/>`, code.Name(), code)
		if !strings.Contains(document, variant) {
			t.Errorf("code %d: %s", code, variant)
		}
		if named, ok := RefusalCodeNamed(code.Name()); !ok || named != code {
			t.Errorf("code %d is not named back by %q", code, code.Name())
		}
	}
}

// A pointer is RFC 6901's: `/` before each reference token, `~` written `~0`
// and `/` written `~1`, `~` escaped first so that `~1` in a name is not read
// back as `/`.
func TestAPointerEscapesTildeAndSolidusInEachToken(t *testing.T) {
	for _, c := range []struct{ prefix, token, want string }{
		{"", "items", "/items"},
		{"/items", "2", "/items/2"},
		{"", "a~b/c", "/a~0b~1c"},
		{"", "~1", "/~01"},
		{"", "", "/"},
	} {
		if got := PointerBelow(c.prefix, c.token); got != c.want {
			t.Errorf("PointerBelow(%q, %q) = %q, want %q", c.prefix, c.token, got, c.want)
		}
	}
}

// A lift refuses a container past its depth over the brackets alone, before the
// text is read as JSON, and names the container it refused, held to the room of
// the refusal's path by whole tokens (docs/adr/0087, sections 2.1 and 2.4).
func TestALiftRefusesAContainerPastItsDepthBeforeItReadsTheText(t *testing.T) {
	for _, c := range []struct {
		text      string
		depth     uint32
		pathBound uint32
		pointer   string
		refused   bool
	}{
		{`{"title":"a","count":1}`, 1, 256, "", false},
		{`{"title":["a"]}`, 1, 256, "/title", true},
		{`{"extra":{"a":1}}`, 1, 256, "/extra", true},
		{`[[1]]`, 1, 256, "/0", true},
		{`[1,2,[3]]`, 1, 256, "/2", true},
		{`{"extra":[[[[[[[`, 1, 256, "/extra", true},
		{`{"title":"[[{\"[","count":1}`, 1, 256, "", false},
		{`{"title":{"a":1}}`, 2, 256, "", false},
		{`{"title":"[[`, 1, 256, "", false},
		{`{"title":"x","extra":[1]}`, 1, 256, "/extra", true},
		{`{"a":{"b":[1]}}`, 2, 256, "/a/b", true},
		{`{"a":[0,{"b":[1]}]}`, 3, 256, "/a/1/b", true},
		{`{"a/b~c":[1]}`, 1, 256, "/a~1b~0c", true},
		{`{[1]}`, 1, 256, "", true},
		{`{"a":{"abcdef":[1]}}`, 2, 4, "/a", true},
		{`{"a":{"bc":[1]}}`, 2, 5, "/a/bc", true},
	} {
		refusal := LiftTooDeep(c.text, c.depth, c.pathBound)
		if (refusal != nil) != c.refused {
			t.Errorf("%s: refused %v, want %v", c.text, refusal, c.refused)
			continue
		}
		if refusal == nil {
			continue
		}
		bound, hasBound := refusal.Bound()
		if refusal.Code() != RefusalTooDeep || refusal.Pointer() != c.pointer || !hasBound || bound != c.depth {
			t.Errorf("%s: refused %s at %q bound %d/%v", c.text, refusal.Code().Name(), refusal.Pointer(), bound, hasBound)
		}
	}
}

// A lift reads its record through the readers a delivery reads one by, a field
// the text lacks taken from the defaults, and answers a refusal as data.
func TestALiftReadsARecordOrAnswersItsRefusal(t *testing.T) {
	read := func(r PayloadRecord) ([2]uint8, error) {
		v, err := LiftField(r, "v", ValueUnsigned[uint8])
		if err != nil {
			return [2]uint8{}, err
		}
		w, err := LiftField(r, "w", ValueUnsigned[uint8])
		return [2]uint8{v, w}, err
	}
	got, refusal := LiftRecord(`{"v":1}`, "Pair", 1, 256, `{"w":7}`, read)
	if refusal != nil || got != [2]uint8{1, 7} {
		t.Fatalf("read %v, refused %v", got, refusal)
	}
	for text, code := range map[string]RefusalCode{
		" \t":             RefusalNotJSON,
		"[[":              RefusalTooDeep,
		"[1]":             RefusalNotObject,
		`{"v":1,"v":2}`:   RefusalNotJSON,
		`{"v":256,"w":1}`: RefusalOutOfRange,
		`{"w":1}`:         RefusalMissing,
	} {
		if _, refusal := LiftRecord(text, "Pair", 1, 256, "", read); refusal == nil || refusal.Code() != code {
			t.Errorf("%q: refused %v, want %s", text, refusal, code.Name())
		}
	}
}

// What a generated reader of an enum field answers for a name its enum does
// not declare: the sentence it always wrote, and the code and pointer beside
// it.
func TestANameNoVariantDeclaresIsRefusedAsUnknownVariant(t *testing.T) {
	err := PayloadUnknownVariant("status", "x", "Status")
	var refusal *PayloadRefusal
	if !errors.As(err, &refusal) {
		t.Fatalf("%v is no refusal", err)
	}
	if refusal.Error() != `"status" (x) is not a variant of Status` {
		t.Errorf("sentence %q", refusal.Error())
	}
	if refusal.Code() != RefusalUnknownVariant || refusal.Pointer() != "/status" {
		t.Errorf("refused %s at %q", refusal.Code().Name(), refusal.Pointer())
	}
	if _, hasBound := refusal.Bound(); hasBound {
		t.Errorf("an unknown variant names no bound")
	}
}
