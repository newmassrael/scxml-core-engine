// SCE-GENERATED — DO NOT EDIT
// source-hash: db91e467a409f671155f800587da9a1dc9cebc08ae59c6337f154a5d7f66ec96
// SCE-MAP: sync_upload_outcome.scxml:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.sync_upload_outcome`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.sync_upload_outcome

fun syncUploadOutcome(create: Boolean, status: Int, davError: Boolean): com.sce.forge.runtime.AlgorithmResult<UByte> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        if (!(status >= 200 && status <= 599)) {
            return com.sce.forge.runtime.AlgorithmResult.Failed(com.sce.forge.runtime.AlgorithmError.Precondition)
        }
        var outcome: UByte = 2.toUByte()
        if (status < 300) {
            outcome = 0.toUByte()
        }
        if (status == 403 || status == 412 || status == 409 && davError) {
            outcome = 1.toUByte()
        }
        if (!create && (status == 404 || status == 410 || status == 409)) {
            outcome = 1.toUByte()
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(outcome)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
