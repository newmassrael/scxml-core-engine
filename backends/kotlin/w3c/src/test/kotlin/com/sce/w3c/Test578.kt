// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test578.scxml:1
package com.sce.w3c

import com.sce.generated.test578.Test578Event
import com.sce.generated.test578.Test578State
import com.sce.generated.test578.Test578StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript data model, if the content provided to populate _event.data cannot be interpreted as
@DisplayName("Test 578 -- W3C SCXML B.2")
class Test578 : W3CTestBase<Test578State, Test578Event>() {
    override fun createStateMachine() = Test578StateMachine(createEngine())
    override val expectedPassState: Test578State = Test578State.Pass
}
