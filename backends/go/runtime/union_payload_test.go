// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A union value is read tag first, then the record of the case its tag names,
// from the same object, and written as one object with its tag among the fields
// of its case (docs/adr/0092, sections 2 and 4).

package sce

import (
	"errors"
	"testing"
)

// figureKind is the tag of figure, read as a generated reader of an enum reads it.
type figureKind uint8

const (
	figureDot figureKind = iota
	figureCircle
	figureDisc
)

// figure is a union over figureKind whose circle and disc cases both hold one
// record, its radius.
type figure struct {
	kind   figureKind
	radius uint16
}

func readFigure(data string) (figure, error) {
	fields, err := LiftPayload(data)
	if err != nil {
		return figure{}, err
	}
	v, err := fields.Field("s")
	if err != nil {
		return figure{}, err
	}
	return ReadUnion(v, "kind", func(tag PayloadValue) (figureKind, error) {
		name, err := tag.Name()
		if err != nil {
			return 0, err
		}
		switch name {
		case "dot":
			return figureDot, nil
		case "circle":
			return figureCircle, nil
		case "disc":
			return figureDisc, nil
		}
		return 0, tag.UnknownVariant(name, "ShapeKind")
	}, func(kind figureKind, object PayloadValue) (figure, error) {
		if kind == figureDot {
			return figure{kind: kind}, nil
		}
		record, err := object.Record("")
		if err != nil {
			return figure{}, err
		}
		radius, err := LiftField(record, "radius", ValueUnsigned[uint16])
		return figure{kind: kind, radius: radius}, err
	})
}

// A union's tag is read first, wherever it stands, and names the case whose
// record is read from the same object: two cases that hold one record stay apart
// by their tag, a key or a null the case does not declare is stepped over, and
// each refusal points at the tag or at the case's field.
func TestAUnionReadsItsTagFirstAndThenTheRecordOfItsCase(t *testing.T) {
	for _, c := range []struct {
		data string
		want figure
	}{
		{`{"s":{"kind":"dot"}}`, figure{kind: figureDot}},
		{`{"s":{"radius":2,"kind":"disc"}}`, figure{kind: figureDisc, radius: 2}},
		{`{"s":{"kind":"circle","radius":2,"width":null,"text":"x"}}`, figure{kind: figureCircle, radius: 2}},
		{`{"s":{"kind":"dot","radius":null}}`, figure{kind: figureDot}},
	} {
		got, err := readFigure(c.data)
		if err != nil || got != c.want {
			t.Errorf("%s: read %+v, %v; want %+v", c.data, got, err, c.want)
		}
	}
	for _, c := range []struct {
		data    string
		code    RefusalCode
		pointer string
	}{
		{`{"s":[]}`, RefusalWrongKind, "/s"},
		{`{"s":null}`, RefusalNullNotOptional, "/s"},
		{`{"s":{"radius":2}}`, RefusalMissing, "/s/kind"},
		{`{"s":{"kind":null,"radius":null}}`, RefusalNullNotOptional, "/s/kind"},
		{`{"s":{"kind":3,"radius":"x"}}`, RefusalWrongKind, "/s/kind"},
		{`{"s":{"kind":"oval","radius":2}}`, RefusalUnknownVariant, "/s/kind"},
		{`{"s":{"kind":"circle","width":2}}`, RefusalMissing, "/s/radius"},
		{`{"s":{"kind":"disc","radius":null}}`, RefusalNullNotOptional, "/s/radius"},
	} {
		_, err := readFigure(c.data)
		var refusal *PayloadRefusal
		if !errors.As(err, &refusal) || refusal.Code() != c.code || refusal.Pointer() != c.pointer {
			t.Errorf("%s: refused %v, want %s at %q", c.data, err, c.code.Name(), c.pointer)
		}
	}
}

// A union is written as one object, its tag among the fields of its case, every
// key ascending by its bytes.
func TestAUnionIsWrittenWithItsTagAmongTheFieldsOfItsCase(t *testing.T) {
	for _, c := range []struct {
		got  RawJSON
		want string
	}{
		{UnionJSON("kind", "none"), `{"kind":"none"}`},
		{UnionJSON("kind", "relativeWeeks", Pair("count", 2)), `{"count":2,"kind":"relativeWeeks"}`},
		{UnionJSON("kind", "absolute", Pair("year", 2027), Pair("month", 3), Pair("day", 3)),
			`{"day":3,"kind":"absolute","month":3,"year":2027}`},
	} {
		if string(c.got) != c.want {
			t.Errorf("a union is written %s, want %s", c.got, c.want)
		}
	}
}
