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
	Name    string `json:"name"`
	Data    string `json:"data"`
	Type    any    `json:"type"`
	Refusal *struct {
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
			_, err := ReadList(v, int(bound), func(element PayloadValue) (struct{}, error) {
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
			fields, err := LiftPayload(c.Data)
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
	if declared := strings.Count(document, "<sce:variant "); declared != int(RefusalNulInText) {
		t.Fatalf("the enum declares %d variants, this runtime %d codes", declared, RefusalNulInText)
	}
	for code := RefusalNotJSON; code <= RefusalNulInText; code++ {
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
