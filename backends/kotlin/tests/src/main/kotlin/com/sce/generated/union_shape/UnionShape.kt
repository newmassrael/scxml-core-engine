// SCE-GENERATED — DO NOT EDIT
// source-hash: 7ab64586d8ea4a589a6a1b1a9889797d0730b91211350d061f6967e67b781d28
// source: union_shape.scxml
// SCE-MAP: union_shape.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="union")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.union_shape

/** The revision of the union: its tag enum's, `enum_shape_kind`'s (a union declares none of its own). */
const val REVISION: UInt = 1u

/** The most bytes a value's JSON text takes: its largest case, every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 108uL

/** The id of the union's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:union_shape:1"

/** A value as a JSON Schema (2020-12): the `oneOf` of the cases, each its tag `kind` beside the fields of its record, every bound as this build resolved it. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:union_shape:1\",\"x-sce-source-hash\":\"7ab64586d8ea4a589a6a1b1a9889797d0730b91211350d061f6967e67b781d28\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"dot\"}},\"required\":[\"kind\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"circle\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"disc\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"rect\"},\"width\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"height\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"width\",\"height\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"label\"},\"text\":{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},\"note\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":4,\"x-sce-max-bytes\":4},{\"type\":\"null\"}]}},\"required\":[\"kind\",\"text\",\"note\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"path\"},\"points\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":3}},\"required\":[\"kind\",\"points\"],\"additionalProperties\":false}]}"

/**
 * A value of `union_shape`: exactly one case, told apart by its tag `kind`, a value of
 * [com.sce.generated.enum_shape_kind.EnumShapeKind]. Two values are equal when they are in one case and their records are equal.
 */
sealed interface UnionShape {
    /** The case this value is in: the tag its text writes under `kind`. */
    val kind: com.sce.generated.enum_shape_kind.EnumShapeKind

    /** The case `dot`, its tag alone. */
    data object Dot : com.sce.generated.union_shape.UnionShape {
        override val kind: com.sce.generated.enum_shape_kind.EnumShapeKind
            get() = com.sce.generated.enum_shape_kind.EnumShapeKind.DOT
    }

    /** The case `circle`, holding its record. */
    data class Circle(val value: com.sce.generated.shape_radius.ShapeRadiusPayload) : com.sce.generated.union_shape.UnionShape {
        override val kind: com.sce.generated.enum_shape_kind.EnumShapeKind
            get() = com.sce.generated.enum_shape_kind.EnumShapeKind.CIRCLE
    }

    /** The case `disc`, holding its record. */
    data class Disc(val value: com.sce.generated.shape_radius.ShapeRadiusPayload) : com.sce.generated.union_shape.UnionShape {
        override val kind: com.sce.generated.enum_shape_kind.EnumShapeKind
            get() = com.sce.generated.enum_shape_kind.EnumShapeKind.DISC
    }

    /** The case `rect`, holding its record. */
    data class Rect(val value: com.sce.generated.shape_rect.ShapeRectPayload) : com.sce.generated.union_shape.UnionShape {
        override val kind: com.sce.generated.enum_shape_kind.EnumShapeKind
            get() = com.sce.generated.enum_shape_kind.EnumShapeKind.RECT
    }

    /** The case `label`, holding its record. */
    data class Label(val value: com.sce.generated.shape_label.ShapeLabelPayload) : com.sce.generated.union_shape.UnionShape {
        override val kind: com.sce.generated.enum_shape_kind.EnumShapeKind
            get() = com.sce.generated.enum_shape_kind.EnumShapeKind.LABEL
    }

    /** The case `path`, holding its record. */
    data class Path(val value: com.sce.generated.shape_path.ShapePathPayload) : com.sce.generated.union_shape.UnionShape {
        override val kind: com.sce.generated.enum_shape_kind.EnumShapeKind
            get() = com.sce.generated.enum_shape_kind.EnumShapeKind.PATH
    }

    /** The record of the case `circle`, read where a test of the tag proved the value is in it. */
    fun provenCircle(): com.sce.generated.shape_radius.ShapeRadiusPayload =
        (this as? Circle)?.value
            ?: throw IllegalStateException("a union_shape read as circle is in ${kind.declaredName()}")

    /** The record of the case `disc`, read where a test of the tag proved the value is in it. */
    fun provenDisc(): com.sce.generated.shape_radius.ShapeRadiusPayload =
        (this as? Disc)?.value
            ?: throw IllegalStateException("a union_shape read as disc is in ${kind.declaredName()}")

    /** The record of the case `rect`, read where a test of the tag proved the value is in it. */
    fun provenRect(): com.sce.generated.shape_rect.ShapeRectPayload =
        (this as? Rect)?.value
            ?: throw IllegalStateException("a union_shape read as rect is in ${kind.declaredName()}")

    /** The record of the case `label`, read where a test of the tag proved the value is in it. */
    fun provenLabel(): com.sce.generated.shape_label.ShapeLabelPayload =
        (this as? Label)?.value
            ?: throw IllegalStateException("a union_shape read as label is in ${kind.declaredName()}")

    /** The record of the case `path`, read where a test of the tag proved the value is in it. */
    fun provenPath(): com.sce.generated.shape_path.ShapePathPayload =
        (this as? Path)?.value
            ?: throw IllegalStateException("a union_shape read as path is in ${kind.declaredName()}")
}
