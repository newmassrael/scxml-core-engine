// SCE-GENERATED — DO NOT EDIT
// source-hash: 1036b5ee37fb5c89aaa1e8156f295f52aeb22d8997a046fafa16b7778f1ace69
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test551.scxml:1
package com.sce.w3c

import com.sce.generated.test551.Test551Event
import com.sce.generated.test551.Test551State
import com.sce.generated.test551.Test551StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.3: f child content is specified, the Platform MUST assign it as the value of the data element at the time specified by the 'binding' attribute of scxml.
@DisplayName("Test 551 -- W3C SCXML 5.3")
class Test551 : W3CTestBase<Test551State, Test551Event>() {
    override fun createStateMachine() = Test551StateMachine(createEngine())
    override val expectedPassState: Test551State = Test551State.Pass
}
