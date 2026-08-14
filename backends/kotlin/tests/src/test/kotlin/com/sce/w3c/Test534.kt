// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b82119528bc210fbc6e453d658ae079f31e3529ce331b1d6045090bb79eaa2ff
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test534.scxml:1
package com.sce.w3c

import com.sce.generated.test534.Test534Event
import com.sce.generated.test534.Test534State
import com.sce.generated.test534.Test534StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: If the 'event' parameter of send is defined, the SCXML Processor MUST use its value as the value of the HTTP POST parameter _scxmleventname
@DisplayName("Test 534 -- W3C SCXML C.2")
class Test534 : W3CHttpTestBase<Test534State, Test534Event>() {
    override fun createStateMachine() = Test534StateMachine(createEngine())
    override val expectedPassState: Test534State = Test534State.Pass
}
