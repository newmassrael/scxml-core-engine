// SCE-GENERATED — DO NOT EDIT
// source-hash: b67ac92c47dabab09bc66be64b0a4b4a6acfabe17f95412102ed640a717fc04f
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
