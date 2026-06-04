// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: f746fc4eb60c4af2ce8afaa281841d74b58f08c5dc3bf4ba795e1c2351ec0f72
// generated-at: 1780577667
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test501.scxml:1
package com.sce.w3c

import com.sce.generated.test501.Test501Event
import com.sce.generated.test501.Test501State
import com.sce.generated.test501.Test501StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: The 'location' field inside the entry for the SCXML Event I/O Processor in the _ioprocessors system variable MUST hold an address that external entities can use to communicate with this SCXML session using the SCXML Event I/O Processor.
@DisplayName("Test 501 -- W3C SCXML C.1")
class Test501 : W3CTestBase<Test501State, Test501Event>() {
    override fun createStateMachine() = Test501StateMachine(createEngine())
    override val expectedPassState: Test501State = Test501State.Pass
}
