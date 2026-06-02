// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: beb72c3a9cb76e61aa4916ff585cb6a1d22e66c189bf8cc96c5023dec391d982
// generated-at: 1780379958
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test339.scxml:1
package com.sce.w3c

import com.sce.generated.test339.Test339Event
import com.sce.generated.test339.Test339State
import com.sce.generated.test339.Test339StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: If an event is not generated from an invoked child process, the Processor MUST leave the invokeid field blank.
@DisplayName("Test 339 -- W3C SCXML 5.10")
class Test339 : W3CTestBase<Test339State, Test339Event>() {
    override fun createStateMachine() = Test339StateMachine()
    override val expectedPassState: Test339State = Test339State.Pass
}
