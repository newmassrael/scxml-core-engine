// SCE-GENERATED — DO NOT EDIT
// source-hash: 50ba4afa70e08ed0ad002509e6baae9623e17ddc8afb219bbba6da905f187359
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
