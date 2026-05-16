// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 9faef2370910e1d1b12ff0b00a3d63d3578977b6f3f2045b8b014f47fa072349
// generated-at: 1778932425
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
