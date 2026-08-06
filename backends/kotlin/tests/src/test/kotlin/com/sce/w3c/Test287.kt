// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: ae61d1957de25e0b25f834d19f5248615526e219f80f117e4ba216dd462396d0
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test287.scxml:1
package com.sce.w3c

import com.sce.generated.test287.Test287Event
import com.sce.generated.test287.Test287State
import com.sce.generated.test287.Test287StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.4: If the location expression of an assign denotes a valid location in the datamodel and if the value specified by 'expr' is a legal value for the location specified, the processor MUST place the specified value at the specified location.
@DisplayName("Test 287 -- W3C SCXML 5.4")
class Test287 : W3CTestBase<Test287State, Test287Event>() {
    override fun createStateMachine() = Test287StateMachine(createEngine())
    override val expectedPassState: Test287State = Test287State.Pass
}
