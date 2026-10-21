// SCE-GENERATED — DO NOT EDIT
// source-hash: 75f0d3c43c82c6af748dd1f1aba98de1b9c4f7c51e43e8850cdf3b0490c95be4
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test448.scxml:1
package com.sce.w3c

import com.sce.generated.test448.Test448Event
import com.sce.generated.test448.Test448State
import com.sce.generated.test448.Test448StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript datamodel, the SCXML Processor must place all variables in a single global ECMAScript scope.
@DisplayName("Test 448 -- W3C SCXML B.2")
class Test448 : W3CTestBase<Test448State, Test448Event>() {
    override fun createStateMachine() = Test448StateMachine(createEngine())
    override val expectedPassState: Test448State = Test448State.Pass
}
