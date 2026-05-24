// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e58c03089e515b4f87df3e09e89234f06d61979361ed8fef1646aeb0069c2169
// generated-at: 1779596481
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test323.scxml:1
package com.sce.w3c

import com.sce.generated.test323.Test323Event
import com.sce.generated.test323.Test323State
import com.sce.generated.test323.Test323StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST bind the variable _name at load time to the value of the 'name' attribute of the scxml element. 	a
@DisplayName("Test 323 -- W3C SCXML 5.10")
class Test323 : W3CTestBase<Test323State, Test323Event>() {
    override fun createStateMachine() = Test323StateMachine(createEngine())
    override val expectedPassState: Test323State = Test323State.Pass
}
