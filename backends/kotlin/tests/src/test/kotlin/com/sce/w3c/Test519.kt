// SCE-GENERATED — DO NOT EDIT
// source-hash: c004aa6d76072d31e9273eed0dc88dc847e351ce7e35f90cc9d56908f01328bd
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
