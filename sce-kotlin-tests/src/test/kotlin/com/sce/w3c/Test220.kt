// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 9c6a4f8dfaed131dd8c4550407375e80f92b4e4373728b55d22f59422722a6ba
// generated-at: 1779372462
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test220.scxml:1
package com.sce.w3c

import com.sce.generated.test220.Test220Event
import com.sce.generated.test220.Test220State
import com.sce.generated.test220.Test220StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: Platforms MUST support http://www.w3.org/TR/scxml/, as a value for the 'type' attribute
@DisplayName("Test 220 -- W3C SCXML 6.4")
class Test220 : W3CTestBase<Test220State, Test220Event>() {
    override fun createStateMachine() = Test220StateMachine()
    override val expectedPassState: Test220State = Test220State.Pass
}
