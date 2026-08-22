// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 465642caa5c7ae5f006b7e4c3302ebaf26878f27c380322c3cf9d87ca35b0ee6
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test309.scxml:1
package com.sce.w3c

import com.sce.generated.test309.Test309Event
import com.sce.generated.test309.Test309State
import com.sce.generated.test309.Test309StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.9: If a conditional expression cannot be evaluated as a boolean value ('true' or 'false') or if its evaluation causes an error, the SCXML processor MUST treat the expression as if it evaluated to 'false'.
@DisplayName("Test 309 -- W3C SCXML 5.9")
class Test309 : W3CTestBase<Test309State, Test309Event>() {
    override fun createStateMachine() = Test309StateMachine(createEngine())
    override val expectedPassState: Test309State = Test309State.Pass
}
