// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d588114b3294b4cb4d7e02d63e6d31a3c0326d3afa0a691deb12b545b5ff5045
// generated-at: 1779460271
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test338.scxml:1
package com.sce.w3c

import com.sce.generated.test338.Test338Event
import com.sce.generated.test338.Test338State
import com.sce.generated.test338.Test338StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: If an event is generated from an invoked child process, the Processor MUST set the invokeid field to the invoke id of the invocation that triggered the child process.
@DisplayName("Test 338 -- W3C SCXML 5.10")
class Test338 : W3CTestBase<Test338State, Test338Event>() {
    override fun createStateMachine() = Test338StateMachine(createEngine())
    override val expectedPassState: Test338State = Test338State.Pass
}
