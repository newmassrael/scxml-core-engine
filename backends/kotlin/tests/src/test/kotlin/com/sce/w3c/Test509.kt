// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: c6c9654e14987bf9fee21998d111ca1385c48c09f2deb9cc862525d124525214
// generated-at: 1785480867
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test509.scxml:1
package com.sce.w3c

import com.sce.generated.test509.Test509Event
import com.sce.generated.test509.Test509State
import com.sce.generated.test509.Test509StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: An SCXML Processor that supports the Basic HTTP Event I/O Processor MUST accept messages at the access URI as HTTP POST requests
@DisplayName("Test 509 -- W3C SCXML C.2")
class Test509 : W3CHttpTestBase<Test509State, Test509Event>() {
    override fun createStateMachine() = Test509StateMachine(createEngine())
    override val expectedPassState: Test509State = Test509State.Pass
}
