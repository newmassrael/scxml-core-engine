// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e58c03089e515b4f87df3e09e89234f06d61979361ed8fef1646aeb0069c2169
// generated-at: 1779596481
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
