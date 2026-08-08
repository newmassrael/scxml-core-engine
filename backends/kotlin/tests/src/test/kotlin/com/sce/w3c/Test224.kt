// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: e9541de728219e5b918752124cad2b5ba2950a5da7bb328f3588c49d2bba35c4
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test224.scxml:1
package com.sce.w3c

import com.sce.generated.test224.Test224Event
import com.sce.generated.test224.Test224State
import com.sce.generated.test224.Test224StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: When the platform generates an identifier for 'idlocation', the identifier MUST have the form stateid.platformid, where stateid is the id of the state containing this element and platformid is automatically generated.
@DisplayName("Test 224 -- W3C SCXML 6.4")
class Test224 : W3CTestBase<Test224State, Test224Event>() {
    override fun createStateMachine() = Test224StateMachine(createEngine())
    override val expectedPassState: Test224State = Test224State.Pass
    override val timeoutMs: Long = 5000L
}
