// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b282d63ae523573aa0c92c912a0dda6cb9508b9193d3508ff15b98a4ec52a48a
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test457.scxml:1
package com.sce.w3c

import com.sce.generated.test457.Test457Event
import com.sce.generated.test457.Test457State
import com.sce.generated.test457.Test457StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript data model, the legal iterable collections are arrays, namely objects that satisfy instanceof(Array) in ECMAScript.  The legal values for the 'item' attribute on foreach are legal ECMAScript variable names.
@DisplayName("Test 457 -- W3C SCXML B.2")
class Test457 : W3CTestBase<Test457State, Test457Event>() {
    override fun createStateMachine() = Test457StateMachine(createEngine())
    override val expectedPassState: Test457State = Test457State.Pass
}
