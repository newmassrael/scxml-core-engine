// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 5b0237a7a83721c40de92b1914fb5f3ab69530a228f19b8f33cd3af4e27ebf24
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test193.scxml:1
package com.sce.w3c

import com.sce.generated.test193.Test193Event
import com.sce.generated.test193.Test193State
import com.sce.generated.test193.Test193StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: [When using the scxml event i/o processor] If neither the 'target' nor the 'targetexpr' attribute is specified, the SCXML Processor MUST add the event to the external event queue of the sending session.
@DisplayName("Test 193 -- W3C SCXML C.1")
class Test193 : W3CTestBase<Test193State, Test193Event>() {
    override fun createStateMachine() = Test193StateMachine()
    override val expectedPassState: Test193State = Test193State.Pass
}
