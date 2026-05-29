// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 00f78dbe00f429352a6571b71d3b75d9ea5e69ddb859956bf6433b48017951ce
// generated-at: 1780031382
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test326.scxml:1
package com.sce.w3c

import com.sce.generated.test326.Test326Event
import com.sce.generated.test326.Test326State
import com.sce.generated.test326.Test326StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST keep the _ioprocessors variable bound to its set of values until the session terminates.
@DisplayName("Test 326 -- W3C SCXML 5.10")
class Test326 : W3CTestBase<Test326State, Test326Event>() {
    override fun createStateMachine() = Test326StateMachine(createEngine())
    override val expectedPassState: Test326State = Test326State.Pass
}
