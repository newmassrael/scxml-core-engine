// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: e273e083fd84459760e6b7e00629aa0bbc396fdd49f2f0b96778152f02d02625
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test354.scxml:1
package com.sce.w3c

import com.sce.generated.test354.Test354Event
import com.sce.generated.test354.Test354State
import com.sce.generated.test354.Test354StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: The 'data' field of the event raised in the receiving session MUST contain a copy of the data specified in the 'namelist' attribute or in param or content elements in the sending session. The nature of the copy operation depends on the datamodel in question. However, the Processor MUST ensure that changes to the transmitted data in the receiving session do not affect the data in the sending session and vice-versa. The format of the 'data' field will depend on the datamodel of the receiving session.
@DisplayName("Test 354 -- W3C SCXML C.1")
class Test354 : W3CTestBase<Test354State, Test354Event>() {
    override fun createStateMachine() = Test354StateMachine(createEngine())
    override val expectedPassState: Test354State = Test354State.Pass
}
