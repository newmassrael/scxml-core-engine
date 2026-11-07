// SCE-GENERATED — DO NOT EDIT
// source-hash: 3e662d3bbddd658a2d3b6f6a012f7bf4d21db1a4e6f0de7d35c1bd71b309f5a0
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
