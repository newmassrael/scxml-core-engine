// SCE-GENERATED — DO NOT EDIT
// source-hash: 1036b5ee37fb5c89aaa1e8156f295f52aeb22d8997a046fafa16b7778f1ace69
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test344.scxml:1
package com.sce.w3c

import com.sce.generated.test344.Test344Event
import com.sce.generated.test344.Test344State
import com.sce.generated.test344.Test344StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.9: If a conditional expression cannot be evaluated as a boolean value ('true' or 'false') or if its evaluation causes an error, the SCXML processor MUST place the error 'error.execution' in the internal event queue.
@DisplayName("Test 344 -- W3C SCXML 5.9")
class Test344 : W3CTestBase<Test344State, Test344Event>() {
    override fun createStateMachine() = Test344StateMachine(createEngine())
    override val expectedPassState: Test344State = Test344State.Pass
}
