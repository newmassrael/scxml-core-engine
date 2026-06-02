// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 32bb8515e09395468fbe442f393d8fa280b19e8eee3f4849a191223ea6d4c265
// generated-at: 1780369943
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test278.scxml:1
package com.sce.w3c

import com.sce.generated.test278.Test278Event
import com.sce.generated.test278.Test278State
import com.sce.generated.test278.Test278StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript data model, the SCXML processor MUST allow any data element to be accessed from any state.
@DisplayName("Test 278 -- W3C SCXML B.2")
class Test278 : W3CTestBase<Test278State, Test278Event>() {
    override fun createStateMachine() = Test278StateMachine(createEngine())
    override val expectedPassState: Test278State = Test278State.Pass
}
