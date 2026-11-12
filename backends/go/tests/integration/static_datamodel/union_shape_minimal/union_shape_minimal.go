// SCE-GENERATED — DO NOT EDIT
// source-hash: 6d71812086a998a80412c256f457bab009adc259010083f54f89e8deddbca71a
// SCE-MAP: union_shape_minimal.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="union")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package union_shape_minimal

import (
	sce_shape_kind "github.com/newmassrael/sce-go-tests/integration/static_datamodel/shape_kind"
	sce_shape_radius_minimal "github.com/newmassrael/sce-go-tests/integration/static_datamodel/shape_radius_minimal"
	sce_shape_rect_minimal "github.com/newmassrael/sce-go-tests/integration/static_datamodel/shape_rect_minimal"
)

// Revision is the revision of the union: its tag enum's, `shape_kind`'s (a union declares none of its own).
const Revision uint32 = 1

// DataMaxLen is the most bytes a value's JSON text takes: its largest case, every list full,
// every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 44

// SchemaID is the id of the union's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:union_shape_minimal:1"

// JSONSchema is a value as a JSON Schema (2020-12): the `oneOf` of the cases, each its tag
// `kind` beside the fields of its record, every bound as this build resolved it.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:union_shape_minimal:1\",\"x-sce-source-hash\":\"6d71812086a998a80412c256f457bab009adc259010083f54f89e8deddbca71a\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"dot\"}},\"required\":[\"kind\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"circle\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"rect\"},\"width\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"height\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"width\",\"height\"],\"additionalProperties\":false}]}"

// UnionShapeMinimal is a value of `union_shape_minimal`: exactly one case, told apart by its tag
// `kind`, a value of sce_shape_kind.ShapeKind. It is built only by the constructor of its case and
// holds a member for the record of each case, so it is copied by value and shares nothing
// with the record it was built from.
type UnionShapeMinimal struct {
	tag sce_shape_kind.ShapeKind
	caseCircle sce_shape_radius_minimal.ShapeRadiusMinimalPayload
	caseRect sce_shape_rect_minimal.ShapeRectMinimalPayload
}

// Kind is the case this value is in: the tag its text writes under `kind`.
func (u UnionShapeMinimal) Kind() sce_shape_kind.ShapeKind {
	return u.tag
}

// UnionShapeMinimalDot is the case `dot`, its tag alone.
func UnionShapeMinimalDot() UnionShapeMinimal {
	return UnionShapeMinimal{tag: sce_shape_kind.ShapeKindDot}
}

// UnionShapeMinimalCircle is the case `circle`, holding its record.
func UnionShapeMinimalCircle(r sce_shape_radius_minimal.ShapeRadiusMinimalPayload) UnionShapeMinimal {
	return UnionShapeMinimal{tag: sce_shape_kind.ShapeKindCircle, caseCircle: r}
}

// UnionShapeMinimalRect is the case `rect`, holding its record.
func UnionShapeMinimalRect(r sce_shape_rect_minimal.ShapeRectMinimalPayload) UnionShapeMinimal {
	return UnionShapeMinimal{tag: sce_shape_kind.ShapeKindRect, caseRect: r}
}

// ProvenCircle is the record of the case `circle`, read where a test of the tag
// proved the value is in it.
func (u UnionShapeMinimal) ProvenCircle() sce_shape_radius_minimal.ShapeRadiusMinimalPayload {
	if u.tag != sce_shape_kind.ShapeKindCircle {
		panic("a value of union_shape_minimal read as circle is in another case")
	}
	return u.caseCircle
}

// ProvenRect is the record of the case `rect`, read where a test of the tag
// proved the value is in it.
func (u UnionShapeMinimal) ProvenRect() sce_shape_rect_minimal.ShapeRectMinimalPayload {
	if u.tag != sce_shape_kind.ShapeKindRect {
		panic("a value of union_shape_minimal read as rect is in another case")
	}
	return u.caseRect
}
