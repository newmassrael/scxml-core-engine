// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 168f4a554705bdfb42cc51a9fbd01e4e5fc028c49c4d6f47071af9577599e075
// generated-at: 1779449862
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test176.scxml:1
package com.sce.w3c

import com.sce.generated.test176.Test176Event
import com.sce.generated.test176.Test176State
import com.sce.generated.test176.Test176StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: The SCXML Processor MUST evaluate param when the parent send element is evaluated and pass the resulting data unmodified to the external service when the message is delivered
@DisplayName("Test 176 -- W3C SCXML 6.2")
class Test176 : W3CTestBase<Test176State, Test176Event>() {
    override fun createStateMachine() = Test176StateMachine(createEngine())
    override val expectedPassState: Test176State = Test176State.Pass
}
