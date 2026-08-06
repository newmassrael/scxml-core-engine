// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 419df244c5f8e83941772fe0e162c3decc43983c72d904462cbbb6425fb07338
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test151.scxml:1
package com.sce.w3c

import com.sce.generated.test151.Test151Event
import com.sce.generated.test151.Test151State
import com.sce.generated.test151.Test151StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.6: In the foreach element, if 'index' is present, the SCXML processor MUST declare a new variable if the one specified by 'index' is not already defined.
@DisplayName("Test 151 -- W3C SCXML 4.6")
class Test151 : W3CTestBase<Test151State, Test151Event>() {
    override fun createStateMachine() = Test151StateMachine(createEngine())
    override val expectedPassState: Test151State = Test151State.Pass
}
