// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: c96808b03e7b119d29792dbf258f9125c91be8c72d4823c8f9b56e0e05a3240b
// generated-at: 0
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
