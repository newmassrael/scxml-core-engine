// SCE-GENERATED — DO NOT EDIT
// source-hash: 6fe63bbaa76604f09a48d24750e75e0a784e90fcb598cdca75c3ac643915bb8d
// SCE-MAP: forge_call_named.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.forge_call_named

data class ValidationResult(val valid: Boolean, val reason: String)

class ForgeCallNamed {

    fun validate(name: String): ValidationResult {
        if (!(name != ""))
            return ValidationResult(false, "plausibility_failed")
        return ValidationResult(true, "")
    }
}
