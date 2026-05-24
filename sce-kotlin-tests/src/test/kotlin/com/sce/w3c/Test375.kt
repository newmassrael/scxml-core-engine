// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e58c03089e515b4f87df3e09e89234f06d61979361ed8fef1646aeb0069c2169
// generated-at: 1779596481
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test375.scxml:1
package com.sce.w3c

import com.sce.generated.test375.Test375Event
import com.sce.generated.test375.Test375State
import com.sce.generated.test375.Test375StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.8: The SCXML processor MUST execute the onentry handlers of a state in document order when the state is entered.
@DisplayName("Test 375 -- W3C SCXML 3.8")
class Test375 : W3CTestBase<Test375State, Test375Event>() {
    override fun createStateMachine() = Test375StateMachine()
    override val expectedPassState: Test375State = Test375State.Pass
}
