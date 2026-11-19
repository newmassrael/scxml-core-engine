// SCE-GENERATED — DO NOT EDIT
// source-hash: c2e56e3837f180f07a5c044369108d947c57fa7c065dd063d76bf9f37c590bee
// SCE-MAP: forge_call_range.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.forge_call_range

data class ValidationResult(val valid: Boolean, val reason: String)

class ForgeCallRange {

    fun validate(level: Int, offset: Int): ValidationResult {
        if (level > 30)
            return ValidationResult(false, "level_out_of_range")
        if (offset < 0)
            return ValidationResult(false, "offset_out_of_range")
        return ValidationResult(true, "")
    }
}
