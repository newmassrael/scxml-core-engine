// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 039ad389d30ffb729c7c2441931b41f36924cbf4b6013115d42ef3094467532b
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test513.scxml:1
package com.sce.w3c

import com.sce.generated.test513.Test513Event
import com.sce.generated.test513.Test513State
import com.sce.generated.test513.Test513StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: After it adds the received message to the appropriate event queue, the SCXML Processor MUST then indicate the result to the external component via a success response code 2XX. (Automated validation: HTTP event received successfully validates 200 OK response)
@DisplayName("Test 513 -- W3C SCXML C.2")
class Test513 : W3CHttpTestBase<Test513State, Test513Event>() {
    override fun createStateMachine() = Test513StateMachine(createEngine())
    override val expectedPassState: Test513State = Test513State.Pass
}
