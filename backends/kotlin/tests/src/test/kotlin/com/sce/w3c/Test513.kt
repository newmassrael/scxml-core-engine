// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 615c09cf1e666fafc78d1f8f6d6f319491336c3f372af9d38785e88a213f5256
// generated-at: 1785425248
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
