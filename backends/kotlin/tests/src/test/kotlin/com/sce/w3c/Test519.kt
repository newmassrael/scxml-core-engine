// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 6d29ccd65cc69c7036210e21d4c9d2a46b7717262dc7e045f86a45620f80383f
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test519.scxml:1
package com.sce.w3c

import com.sce.generated.test519.Test519Event
import com.sce.generated.test519.Test519State
import com.sce.generated.test519.Test519StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: If one or more param children are present [in send], the SCXML Processor MUST map their names (i.e. name attributes) and values to HTTP POST parameters
@DisplayName("Test 519 -- W3C SCXML C.2")
class Test519 : W3CHttpTestBase<Test519State, Test519Event>() {
    override fun createStateMachine() = Test519StateMachine(createEngine())
    override val expectedPassState: Test519State = Test519State.Pass
}
