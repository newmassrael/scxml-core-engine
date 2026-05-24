// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e58c03089e515b4f87df3e09e89234f06d61979361ed8fef1646aeb0069c2169
// generated-at: 1779596481
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test355.scxml:1
package com.sce.w3c

import com.sce.generated.test355.Test355Event
import com.sce.generated.test355.Test355State
import com.sce.generated.test355.Test355StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.2: At system initialization time, if the 'initial' attribute is not present, the Processor MUST enter the first state in document order.
@DisplayName("Test 355 -- W3C SCXML 3.2")
class Test355 : W3CTestBase<Test355State, Test355Event>() {
    override fun createStateMachine() = Test355StateMachine()
    override val expectedPassState: Test355State = Test355State.Pass
}
