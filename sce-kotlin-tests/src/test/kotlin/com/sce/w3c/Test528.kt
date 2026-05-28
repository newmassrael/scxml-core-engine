// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d9c7eeffd42250afac7bb84392f7db6b4e0a95d9e7e2e16957a4ecc188fd0aa8
// generated-at: 1779980218
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test528.scxml:1
package com.sce.w3c

import com.sce.generated.test528.Test528Event
import com.sce.generated.test528.Test528State
import com.sce.generated.test528.Test528StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.6: f the evaluation of 'expr' produces an error, the Processor MUST place error.execution in the internal event queue and use the empty string as the output of the content element.
@DisplayName("Test 528 -- W3C SCXML 5.6")
class Test528 : W3CTestBase<Test528State, Test528Event>() {
    override fun createStateMachine() = Test528StateMachine(createEngine())
    override val expectedPassState: Test528State = Test528State.Pass
}
