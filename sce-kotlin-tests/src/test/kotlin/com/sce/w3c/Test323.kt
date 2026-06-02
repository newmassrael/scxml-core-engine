// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 32bb8515e09395468fbe442f393d8fa280b19e8eee3f4849a191223ea6d4c265
// generated-at: 1780369943
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
