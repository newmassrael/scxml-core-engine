// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 0df7c3dd89bf1ab35c62dca175cae2bb2e377b70fda63f4fb76009a06edcd3df
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test531.scxml:1
package com.sce.w3c

import com.sce.generated.test531.Test531Event
import com.sce.generated.test531.Test531State
import com.sce.generated.test531.Test531StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: If a single instance of the parameter '_scxmleventname' is present, the SCXML Processor MUST use its value as the name of the SCXML event that it raises.
@DisplayName("Test 531 -- W3C SCXML C.2")
class Test531 : W3CHttpTestBase<Test531State, Test531Event>() {
    override fun createStateMachine() = Test531StateMachine(createEngine())
    override val expectedPassState: Test531State = Test531State.Pass
}
