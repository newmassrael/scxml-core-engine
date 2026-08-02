// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 566d82cde8067d5a043ddb08a09857bfebb8c9df80a7d6c2995a193c1455a335
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test220.scxml:1
package com.sce.w3c

import com.sce.generated.test220.Test220Event
import com.sce.generated.test220.Test220State
import com.sce.generated.test220.Test220StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: Platforms MUST support http://www.w3.org/TR/scxml/, as a value for the 'type' attribute
@DisplayName("Test 220 -- W3C SCXML 6.4")
class Test220 : W3CTestBase<Test220State, Test220Event>() {
    override fun createStateMachine() = Test220StateMachine()
    override val expectedPassState: Test220State = Test220State.Pass
}
