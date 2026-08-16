// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 128f5bda1db8a8695e204b38e87b8d2d3815bdde9691186823a5ecdc7374af1d
// generated-at: 0
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
