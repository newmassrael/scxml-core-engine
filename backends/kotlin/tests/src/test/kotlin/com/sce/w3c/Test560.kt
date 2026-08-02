// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 566d82cde8067d5a043ddb08a09857bfebb8c9df80a7d6c2995a193c1455a335
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test560.scxml:1
package com.sce.w3c

import com.sce.generated.test560.Test560Event
import com.sce.generated.test560.Test560State
import com.sce.generated.test560.Test560StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript data model, if the content provided to populate _event.data can be interpeted as key-value pairs, then for each unique key, the SCXML Processor MUST create a property of _event.data whose name is the name of the key-value pair and whose value is the value of the key-value pair.
@DisplayName("Test 560 -- W3C SCXML B.2")
class Test560 : W3CTestBase<Test560State, Test560Event>() {
    override fun createStateMachine() = Test560StateMachine(createEngine())
    override val expectedPassState: Test560State = Test560State.Pass
}
