package com.khmerime

import com.khmerime.input.KhmerImeSession
import com.khmerime.input.KhmerRenderState
import com.khmerime.input.ModelRefiner
import org.junit.Assert.*
import org.junit.Test

// ModelRefinerBehaviorTest
// ========================
// Deep-module behavior for the debounced, off-hot-path model refine. Real Rust
// session via JNI, manual RecordingDispatcher so debounce + cancellation are
// deterministic. Mirrors the iOS Visible Refiner behavior.

class ModelRefinerBehaviorTest {

    private fun makeRefiner(
        dispatcher: RecordingDispatcher,
        onRefined: (KhmerRenderState) -> Unit = {},
    ): Pair<ModelRefiner, KhmerImeSession> {
        val session = KhmerImeSession()
        session.setModelMode(true)
        val refiner = ModelRefiner(session, dispatcher, onRefined)
        return Pair(refiner, session)
    }

    @Test
    fun scheduleAfterDebounceRefinesAndReportsResult() {
        val dispatcher = RecordingDispatcher()
        var rendered: KhmerRenderState? = null
        val (refiner, _) = makeRefiner(dispatcher) { rendered = it }

        refiner.schedule("nhom")
        assertNull("must not refine before the debounce elapses", rendered)

        dispatcher.runPendingDelayed()

        assertNotNull("refine must report a render state once the debounce fires", rendered)
    }

    @Test
    fun rapidSchedulesDebounceToASingleRefine() {
        val dispatcher = RecordingDispatcher()
        var refines = 0
        val (refiner, _) = makeRefiner(dispatcher) { refines++ }

        refiner.schedule("nh")
        refiner.schedule("nho")
        refiner.schedule("nhom")
        dispatcher.runPendingDelayed()

        assertEquals("only the last scheduled refine should survive the debounce", 1, refines)
    }

    @Test
    fun cancelDropsThePendingRefine() {
        val dispatcher = RecordingDispatcher()
        var refines = 0
        val (refiner, _) = makeRefiner(dispatcher) { refines++ }

        refiner.schedule("nhom")
        refiner.cancel()
        dispatcher.runPendingDelayed()

        assertEquals("a cancelled refine must never report a result", 0, refines)
    }
}
