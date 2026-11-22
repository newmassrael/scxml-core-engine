// SCE-GENERATED — DO NOT EDIT
// source-hash: 02b88167d0d255fce6429b7db181f1e11b9af8be1c1f7c3ab0ba30a3958c5a02
// source: grow_shape_1.scxml
// SCE-MAP: grow_shape_1.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="union")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package grow_shape_1

import (
	sce_grow_kind "github.com/newmassrael/sce-go-tests/integration/static_datamodel/grow_kind"
	sce_grow_inner_1 "github.com/newmassrael/sce-go-tests/integration/static_datamodel/grow_inner_1"
)

// Revision is the revision of the union: its tag enum's, `grow_kind`'s (a union declares none of its own).
const Revision uint32 = 1

// DataMaxLen is the most bytes a value's JSON text takes: its largest case, every list full,
// every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 27

// SchemaID is the id of the union's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:grow_shape_1:1"

// JSONSchema is a value as a JSON Schema (2020-12): the `oneOf` of the cases, each its tag
// `kind` beside the fields of its record, every bound as this build resolved it.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:grow_shape_1:1\",\"x-sce-source-hash\":\"02b88167d0d255fce6429b7db181f1e11b9af8be1c1f7c3ab0ba30a3958c5a02\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"single\"},\"n\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"n\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"none\"}},\"required\":[\"kind\"],\"additionalProperties\":false}]}"

// GrowShape1 is a value of `grow_shape_1`: exactly one case, told apart by its tag
// `kind`, a value of sce_grow_kind.GrowKind. It is built only by the constructor of its case and
// holds a member for the record of each case, so it is copied by value and shares nothing
// with the record it was built from.
type GrowShape1 struct {
	tag sce_grow_kind.GrowKind
	caseSingle sce_grow_inner_1.GrowInner1Payload
}

// Kind is the case this value is in: the tag its text writes under `kind`.
func (u GrowShape1) Kind() sce_grow_kind.GrowKind {
	return u.tag
}

// GrowShape1Single is the case `single`, holding its record.
func GrowShape1Single(r sce_grow_inner_1.GrowInner1Payload) GrowShape1 {
	return GrowShape1{tag: sce_grow_kind.GrowKindSingle, caseSingle: r}
}

// GrowShape1None is the case `none`, its tag alone.
func GrowShape1None() GrowShape1 {
	return GrowShape1{tag: sce_grow_kind.GrowKindNone}
}

// ProvenSingle is the record of the case `single`, read where a test of the tag
// proved the value is in it.
func (u GrowShape1) ProvenSingle() sce_grow_inner_1.GrowInner1Payload {
	if u.tag != sce_grow_kind.GrowKindSingle {
		panic("a value of grow_shape_1 read as single is in another case")
	}
	return u.caseSingle
}
