// SCE-GENERATED — DO NOT EDIT
// source-hash: 02b88167d0d255fce6429b7db181f1e11b9af8be1c1f7c3ab0ba30a3958c5a02
// source: grow_shape_1.scxml
// SCE-MAP: grow_shape_1.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="union")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.grow_shape_1

/** The revision of the union: its tag enum's, `grow_kind`'s (a union declares none of its own). */
const val REVISION: UInt = 1u

/** The most bytes a value's JSON text takes: its largest case, every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 27uL

/** The id of the union's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:grow_shape_1:1"

/** A value as a JSON Schema (2020-12): the `oneOf` of the cases, each its tag `kind` beside the fields of its record, every bound as this build resolved it. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:grow_shape_1:1\",\"x-sce-source-hash\":\"02b88167d0d255fce6429b7db181f1e11b9af8be1c1f7c3ab0ba30a3958c5a02\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"single\"},\"n\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"n\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"none\"}},\"required\":[\"kind\"],\"additionalProperties\":false}]}"

/**
 * A value of `grow_shape_1`: exactly one case, told apart by its tag `kind`, a value of
 * [com.sce.generated.grow_kind.GrowKind]. Two values are equal when they are in one case and their records are equal.
 */
sealed interface GrowShape1 {
    /** The case this value is in: the tag its text writes under `kind`. */
    val kind: com.sce.generated.grow_kind.GrowKind

    /** The case `single`, holding its record. */
    data class Single(val value: com.sce.generated.grow_inner_1.GrowInner1Payload) : com.sce.generated.grow_shape_1.GrowShape1 {
        override val kind: com.sce.generated.grow_kind.GrowKind
            get() = com.sce.generated.grow_kind.GrowKind.SINGLE
    }

    /** The case `none`, its tag alone. */
    data object None : com.sce.generated.grow_shape_1.GrowShape1 {
        override val kind: com.sce.generated.grow_kind.GrowKind
            get() = com.sce.generated.grow_kind.GrowKind.NONE
    }

    /** The record of the case `single`, read where a test of the tag proved the value is in it. */
    fun provenSingle(): com.sce.generated.grow_inner_1.GrowInner1Payload =
        (this as? Single)?.value
            ?: throw IllegalStateException("a grow_shape_1 read as single is in ${kind.declaredName()}")
}
