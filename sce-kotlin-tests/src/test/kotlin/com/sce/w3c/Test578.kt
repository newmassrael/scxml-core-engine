// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 00f78dbe00f429352a6571b71d3b75d9ea5e69ddb859956bf6433b48017951ce
// generated-at: 1780031382
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
