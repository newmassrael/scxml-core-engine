// SCE-GENERATED — DO NOT EDIT
// source-hash: ca3b055deba6b38537c3642140f701b5f98f1b971d59425d3c780765d8a6a19b
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
