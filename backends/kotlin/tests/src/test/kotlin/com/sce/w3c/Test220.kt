// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 5b0237a7a83721c40de92b1914fb5f3ab69530a228f19b8f33cd3af4e27ebf24
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
