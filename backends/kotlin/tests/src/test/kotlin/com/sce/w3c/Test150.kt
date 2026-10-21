// SCE-GENERATED — DO NOT EDIT
// source-hash: 75f0d3c43c82c6af748dd1f1aba98de1b9c4f7c51e43e8850cdf3b0490c95be4
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test150.scxml:1
package com.sce.w3c

import com.sce.generated.test150.Test150Event
import com.sce.generated.test150.Test150State
import com.sce.generated.test150.Test150StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.6: In the foreach element, the SCXML processor MUST declare a new variable if the one specified by 'item' is not already defined.
@DisplayName("Test 150 -- W3C SCXML 4.6")
class Test150 : W3CTestBase<Test150State, Test150Event>() {
    override fun createStateMachine() = Test150StateMachine(createEngine())
    override val expectedPassState: Test150State = Test150State.Pass
}
