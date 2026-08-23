// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: c44eb8ea1f7a6700f381c20ea1b37f015805c8beff30d4d12e22d7c96e5e1124
// generated-at: 0
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
