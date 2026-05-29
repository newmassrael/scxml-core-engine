// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 00f78dbe00f429352a6571b71d3b75d9ea5e69ddb859956bf6433b48017951ce
// generated-at: 1780031382
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test210.scxml:1
package com.sce.w3c

import com.sce.generated.test210.Test210Event
import com.sce.generated.test210.Test210State
import com.sce.generated.test210.Test210StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.3: If the 'sendidexpr' attribute is present, the SCXML Processor MUST evaluate it when the parent cancel element is evaluated and treat the result as if it had been entered as the value of 'sendid'.
@DisplayName("Test 210 -- W3C SCXML 6.3")
class Test210 : W3CTestBase<Test210State, Test210Event>() {
    override fun createStateMachine() = Test210StateMachine(createEngine())
    override val expectedPassState: Test210State = Test210State.Pass
    override val timeoutMs: Long = 5000L
}
