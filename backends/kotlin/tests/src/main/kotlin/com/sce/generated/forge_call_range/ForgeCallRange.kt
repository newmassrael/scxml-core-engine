// SCE-GENERATED — DO NOT EDIT
// source-hash: ea85fee4970d20335e2c3e2f4a0cd89a9f4f0a21d54f6dae4cb7a7218e2be4b5
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
