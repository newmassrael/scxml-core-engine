// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: merkle_node.scxml:32 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: merge.merkle_node
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.merkle_node

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "merge.merkle_node"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

data class MerkleNodePayload(    val depth: UByte,    val prefix: ULong,    val hash: UInt)
