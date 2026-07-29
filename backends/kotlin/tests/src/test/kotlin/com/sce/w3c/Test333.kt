// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c22d767976ad0f3af27597215acac4daa969b18394744727f9f1e4af8f5db2d7
// generated-at: 1785338317
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test333.scxml:1
package com.sce.w3c

import com.sce.generated.test333.Test333Event
import com.sce.generated.test333.Test333State
import com.sce.generated.test333.Test333StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: For events other than error events triggered by a failed attempt to send an event, if the sending entity
@DisplayName("Test 333 -- W3C SCXML 5.10")
class Test333 : W3CTestBase<Test333State, Test333Event>() {
    override fun createStateMachine() = Test333StateMachine()
    override val expectedPassState: Test333State = Test333State.Pass
}
