// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 30c21c2126baf025b95abcba3754b0bcf0280066f6d16a0568643a49c1942e1f
// generated-at: 1779967138
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
