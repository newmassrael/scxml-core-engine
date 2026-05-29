// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 00f78dbe00f429352a6571b71d3b75d9ea5e69ddb859956bf6433b48017951ce
// generated-at: 1780031382
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test550.scxml:1
package com.sce.w3c

import com.sce.generated.test550.Test550Event
import com.sce.generated.test550.Test550State
import com.sce.generated.test550.Test550StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.3: If the 'expr' attribute is present, the Platform MUST evaluate the corresponding expression at the time specified by the 'binding' attribute of scxml and MUST assign the resulting value as the value of the data element
@DisplayName("Test 550 -- W3C SCXML 5.3")
class Test550 : W3CTestBase<Test550State, Test550Event>() {
    override fun createStateMachine() = Test550StateMachine(createEngine())
    override val expectedPassState: Test550State = Test550State.Pass
}
