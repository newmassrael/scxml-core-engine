// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c496f893fb4def171deba817f047a2a335356d181c631fa74825a157a7412c3e
// generated-at: 1784370263
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test346.scxml:1
package com.sce.w3c

import com.sce.generated.test346.Test346Event
import com.sce.generated.test346.Test346State
import com.sce.generated.test346.Test346StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST place the error error.execution on the internal event queue when any attempt to change the value of a system variable is made.
@DisplayName("Test 346 -- W3C SCXML 5.10")
class Test346 : W3CTestBase<Test346State, Test346Event>() {
    override fun createStateMachine() = Test346StateMachine(createEngine())
    override val expectedPassState: Test346State = Test346State.Pass
}
