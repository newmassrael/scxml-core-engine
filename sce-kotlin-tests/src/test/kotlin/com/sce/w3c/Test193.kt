// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: bc7b5b1dd90f65e6c3a4df2e3c4223cf8922d7e6b2d5d124b66683d16074cb6e
// generated-at: 1780362263
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
