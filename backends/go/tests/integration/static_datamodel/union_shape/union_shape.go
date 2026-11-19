// SCE-GENERATED — DO NOT EDIT
// source-hash: 7ab64586d8ea4a589a6a1b1a9889797d0730b91211350d061f6967e67b781d28
// source: union_shape.scxml
// SCE-MAP: union_shape.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="union")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package union_shape

import (
	sce_enum_shape_kind "github.com/newmassrael/sce-go-tests/integration/static_datamodel/enum_shape_kind"
	sce_shape_radius "github.com/newmassrael/sce-go-tests/integration/static_datamodel/shape_radius"
	sce_shape_rect "github.com/newmassrael/sce-go-tests/integration/static_datamodel/shape_rect"
	sce_shape_label "github.com/newmassrael/sce-go-tests/integration/static_datamodel/shape_label"
	sce_shape_path "github.com/newmassrael/sce-go-tests/integration/static_datamodel/shape_path"
)

// Revision is the revision of the union: its tag enum's, `enum_shape_kind`'s (a union declares none of its own).
const Revision uint32 = 1

// DataMaxLen is the most bytes a value's JSON text takes: its largest case, every list full,
// every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 108

// SchemaID is the id of the union's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:union_shape:1"

// JSONSchema is a value as a JSON Schema (2020-12): the `oneOf` of the cases, each its tag
// `kind` beside the fields of its record, every bound as this build resolved it.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:union_shape:1\",\"x-sce-source-hash\":\"7ab64586d8ea4a589a6a1b1a9889797d0730b91211350d061f6967e67b781d28\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"dot\"}},\"required\":[\"kind\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"circle\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"disc\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"rect\"},\"width\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"height\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"width\",\"height\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"label\"},\"text\":{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},\"note\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":4,\"x-sce-max-bytes\":4},{\"type\":\"null\"}]}},\"required\":[\"kind\",\"text\",\"note\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"path\"},\"points\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":3}},\"required\":[\"kind\",\"points\"],\"additionalProperties\":false}]}"

// UnionShape is a value of `union_shape`: exactly one case, told apart by its tag
// `kind`, a value of sce_enum_shape_kind.EnumShapeKind. It is built only by the constructor of its case and
// holds a member for the record of each case, so it is copied by value and shares nothing
// with the record it was built from.
type UnionShape struct {
	tag sce_enum_shape_kind.EnumShapeKind
	caseCircle sce_shape_radius.ShapeRadiusPayload
	caseDisc sce_shape_radius.ShapeRadiusPayload
	caseRect sce_shape_rect.ShapeRectPayload
	caseLabel sce_shape_label.ShapeLabelPayload
	casePath sce_shape_path.ShapePathPayload
}

// Kind is the case this value is in: the tag its text writes under `kind`.
func (u UnionShape) Kind() sce_enum_shape_kind.EnumShapeKind {
	return u.tag
}

// UnionShapeDot is the case `dot`, its tag alone.
func UnionShapeDot() UnionShape {
	return UnionShape{tag: sce_enum_shape_kind.EnumShapeKindDot}
}

// UnionShapeCircle is the case `circle`, holding its record.
func UnionShapeCircle(r sce_shape_radius.ShapeRadiusPayload) UnionShape {
	return UnionShape{tag: sce_enum_shape_kind.EnumShapeKindCircle, caseCircle: r}
}

// UnionShapeDisc is the case `disc`, holding its record.
func UnionShapeDisc(r sce_shape_radius.ShapeRadiusPayload) UnionShape {
	return UnionShape{tag: sce_enum_shape_kind.EnumShapeKindDisc, caseDisc: r}
}

// UnionShapeRect is the case `rect`, holding its record.
func UnionShapeRect(r sce_shape_rect.ShapeRectPayload) UnionShape {
	return UnionShape{tag: sce_enum_shape_kind.EnumShapeKindRect, caseRect: r}
}

// UnionShapeLabel is the case `label`, holding its record.
func UnionShapeLabel(r sce_shape_label.ShapeLabelPayload) UnionShape {
	return UnionShape{tag: sce_enum_shape_kind.EnumShapeKindLabel, caseLabel: r}
}

// UnionShapePath is the case `path`, holding its record.
func UnionShapePath(r sce_shape_path.ShapePathPayload) UnionShape {
	return UnionShape{tag: sce_enum_shape_kind.EnumShapeKindPath, casePath: r}
}

// ProvenCircle is the record of the case `circle`, read where a test of the tag
// proved the value is in it.
func (u UnionShape) ProvenCircle() sce_shape_radius.ShapeRadiusPayload {
	if u.tag != sce_enum_shape_kind.EnumShapeKindCircle {
		panic("a value of union_shape read as circle is in another case")
	}
	return u.caseCircle
}

// ProvenDisc is the record of the case `disc`, read where a test of the tag
// proved the value is in it.
func (u UnionShape) ProvenDisc() sce_shape_radius.ShapeRadiusPayload {
	if u.tag != sce_enum_shape_kind.EnumShapeKindDisc {
		panic("a value of union_shape read as disc is in another case")
	}
	return u.caseDisc
}

// ProvenRect is the record of the case `rect`, read where a test of the tag
// proved the value is in it.
func (u UnionShape) ProvenRect() sce_shape_rect.ShapeRectPayload {
	if u.tag != sce_enum_shape_kind.EnumShapeKindRect {
		panic("a value of union_shape read as rect is in another case")
	}
	return u.caseRect
}

// ProvenLabel is the record of the case `label`, read where a test of the tag
// proved the value is in it.
func (u UnionShape) ProvenLabel() sce_shape_label.ShapeLabelPayload {
	if u.tag != sce_enum_shape_kind.EnumShapeKindLabel {
		panic("a value of union_shape read as label is in another case")
	}
	return u.caseLabel
}

// ProvenPath is the record of the case `path`, read where a test of the tag
// proved the value is in it.
func (u UnionShape) ProvenPath() sce_shape_path.ShapePathPayload {
	if u.tag != sce_enum_shape_kind.EnumShapeKindPath {
		panic("a value of union_shape read as path is in another case")
	}
	return u.casePath
}
