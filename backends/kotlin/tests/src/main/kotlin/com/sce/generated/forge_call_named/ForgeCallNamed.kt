// SCE-GENERATED — DO NOT EDIT
// source-hash: d8c75e4d0233623996dc9a7cc3bdbe850af8d4f91ab023212ebd9f938cf7955f
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
