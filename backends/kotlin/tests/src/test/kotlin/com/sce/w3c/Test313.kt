// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 903bfb24c21707102bb3eb8f65796f065ff471e2b1842192d62344bdbecfb856
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test313.scxml:1
package com.sce.w3c

import com.sce.generated.test313.Test313Event
import com.sce.generated.test313.Test313State
import com.sce.generated.test313.Test313StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.9: The SCXML processor MAY reject documents containing syntactically ill-formed expressions at document load time, or it MAY wait and place error.execution in the internal event queue at runtime when the expressions are evaluated.
@DisplayName("Test 313 -- W3C SCXML 5.9")
class Test313 : W3CTestBase<Test313State, Test313Event>() {
    override fun createStateMachine() = Test313StateMachine(createEngine())
    override val expectedPassState: Test313State = Test313State.Pass
}
