// SCE-GENERATED — DO NOT EDIT
// source-hash: 1036b5ee37fb5c89aaa1e8156f295f52aeb22d8997a046fafa16b7778f1ace69
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
