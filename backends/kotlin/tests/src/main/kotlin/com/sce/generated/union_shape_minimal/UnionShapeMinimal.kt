// SCE-GENERATED — DO NOT EDIT
// source-hash: 6d71812086a998a80412c256f457bab009adc259010083f54f89e8deddbca71a
// SCE-MAP: union_shape_minimal.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="union")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.union_shape_minimal

/** The revision of the union: its tag enum's, `shape_kind`'s (a union declares none of its own). */
const val REVISION: UInt = 1u

/** The most bytes a value's JSON text takes: its largest case, every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 44uL

/** The id of the union's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:union_shape_minimal:1"

/** A value as a JSON Schema (2020-12): the `oneOf` of the cases, each its tag `kind` beside the fields of its record, every bound as this build resolved it. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:union_shape_minimal:1\",\"x-sce-source-hash\":\"6d71812086a998a80412c256f457bab009adc259010083f54f89e8deddbca71a\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"dot\"}},\"required\":[\"kind\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"circle\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"rect\"},\"width\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"height\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"width\",\"height\"],\"additionalProperties\":false}]}"

/**
 * A value of `union_shape_minimal`: exactly one case, told apart by its tag `kind`, a value of
 * [com.sce.generated.shape_kind.ShapeKind]. Two values are equal when they are in one case and their records are equal.
 */
sealed interface UnionShapeMinimal {
    /** The case this value is in: the tag its text writes under `kind`. */
    val kind: com.sce.generated.shape_kind.ShapeKind

    /** The case `dot`, its tag alone. */
    data object Dot : com.sce.generated.union_shape_minimal.UnionShapeMinimal {
        override val kind: com.sce.generated.shape_kind.ShapeKind
            get() = com.sce.generated.shape_kind.ShapeKind.DOT
    }

    /** The case `circle`, holding its record. */
    data class Circle(val value: com.sce.generated.shape_radius_minimal.ShapeRadiusMinimalPayload) : com.sce.generated.union_shape_minimal.UnionShapeMinimal {
        override val kind: com.sce.generated.shape_kind.ShapeKind
            get() = com.sce.generated.shape_kind.ShapeKind.CIRCLE
    }

    /** The case `rect`, holding its record. */
    data class Rect(val value: com.sce.generated.shape_rect_minimal.ShapeRectMinimalPayload) : com.sce.generated.union_shape_minimal.UnionShapeMinimal {
        override val kind: com.sce.generated.shape_kind.ShapeKind
            get() = com.sce.generated.shape_kind.ShapeKind.RECT
    }

    /** The record of the case `circle`, read where a test of the tag proved the value is in it. */
    fun provenCircle(): com.sce.generated.shape_radius_minimal.ShapeRadiusMinimalPayload =
        (this as? Circle)?.value
            ?: throw IllegalStateException("a union_shape_minimal read as circle is in ${kind.declaredName()}")

    /** The record of the case `rect`, read where a test of the tag proved the value is in it. */
    fun provenRect(): com.sce.generated.shape_rect_minimal.ShapeRectMinimalPayload =
        (this as? Rect)?.value
            ?: throw IllegalStateException("a union_shape_minimal read as rect is in ${kind.declaredName()}")
}
