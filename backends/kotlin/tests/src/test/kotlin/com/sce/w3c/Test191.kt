// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 0df7c3dd89bf1ab35c62dca175cae2bb2e377b70fda63f4fb76009a06edcd3df
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test191.scxml:1
package com.sce.w3c

import com.sce.generated.test191.Test191Event
import com.sce.generated.test191.Test191State
import com.sce.generated.test191.Test191StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: [When using the scxml event i/o processor] If the target is the special term '#_parent', the Processor MUST add the event to the external event queue of the SCXML session that invoked the sending session, if there is one.
@DisplayName("Test 191 -- W3C SCXML C.1")
class Test191 : W3CTestBase<Test191State, Test191Event>() {
    override fun createStateMachine() = Test191StateMachine()
    override val expectedPassState: Test191State = Test191State.Pass
    override val timeoutMs: Long = 5000L
}
