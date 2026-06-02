// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 32bb8515e09395468fbe442f393d8fa280b19e8eee3f4849a191223ea6d4c265
// generated-at: 1780369943
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test319.scxml:1
package com.sce.w3c

import com.sce.generated.test319.Test319Event
import com.sce.generated.test319.Test319State
import com.sce.generated.test319.Test319StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The SCXML Processor MUST NOT bind _event at initialization time until the first event is processed.
@DisplayName("Test 319 -- W3C SCXML 5.10")
class Test319 : W3CTestBase<Test319State, Test319Event>() {
    override fun createStateMachine() = Test319StateMachine(createEngine())
    override val expectedPassState: Test319State = Test319State.Pass
}
