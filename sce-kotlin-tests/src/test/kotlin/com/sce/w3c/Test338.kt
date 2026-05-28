// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d9c7eeffd42250afac7bb84392f7db6b4e0a95d9e7e2e16957a4ecc188fd0aa8
// generated-at: 1779980218
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
