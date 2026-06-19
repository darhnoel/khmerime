package com.khmerime.views

import org.junit.Assert.*
import org.junit.Test

// ViewPoolTest
// ============
// The pool's grow/hide-excess policy is the only real logic here, so it's
// tested against a fake child type (String) with recording lambdas instead
// of real Android views — no Robolectric needed.

class ViewPoolTest {

    private fun makePool(): Triple<ViewPool<String>, MutableList<String>, MutableMap<String, Boolean>> {
        val added = mutableListOf<String>()
        val visibility = mutableMapOf<String, Boolean>()
        var nextId = 0
        val pool = ViewPool<String>(
            createChild = { "child${nextId++}" },
            addChild = { added.add(it) },
            setVisible = { child, visible -> visibility[child] = visible },
        )
        return Triple(pool, added, visibility)
    }

    @Test
    fun syncCreatesAndAddsChildrenUpToRequestedCount() {
        val (pool, added, _) = makePool()

        val visible = pool.sync(3)

        assertEquals(listOf("child0", "child1", "child2"), visible)
        assertEquals("each new child must be added to the parent exactly once", 3, added.size)
    }

    @Test
    fun syncWithLowerCountHidesExcessInsteadOfRemoving() {
        val (pool, added, visibility) = makePool()
        pool.sync(3)

        val visible = pool.sync(1)

        assertEquals(listOf("child0"), visible)
        assertEquals("no children should be removed, only hidden", 3, added.size)
        assertEquals(true, visibility["child0"])
        assertEquals(false, visibility["child1"])
        assertEquals(false, visibility["child2"])
    }

    @Test
    fun syncGrowingAgainReusesPreviouslyHiddenChildren() {
        val (pool, added, visibility) = makePool()
        pool.sync(3)
        pool.sync(1)

        val visible = pool.sync(3)

        assertEquals(listOf("child0", "child1", "child2"), visible)
        assertEquals("growing back up must reuse existing children, not create new ones", 3, added.size)
        assertEquals(true, visibility["child1"])
        assertEquals(true, visibility["child2"])
    }

    // clear() exists so the input view can be torn down without the
    // service-scoped pool pinning the destroyed view hierarchy (memory leak
    // across onCreateInputView/onDestroyInputView). It must detach every
    // pooled child from its parent and drop the references, so a later sync
    // rebuilds against the freshly created input view.

    @Test
    fun clearRemovesEveryChildFromParentAndDropsReferences() {
        val added = mutableListOf<String>()
        val removed = mutableListOf<String>()
        var nextId = 0
        val pool = ViewPool<String>(
            createChild = { "child${nextId++}" },
            addChild = { added.add(it) },
            setVisible = { _, _ -> },
            removeChild = { removed.add(it) },
        )
        pool.sync(3)

        pool.clear()

        assertEquals(
            "clear must detach every pooled child from its parent",
            listOf("child0", "child1", "child2"),
            removed,
        )

        val visible = pool.sync(2)
        assertEquals(
            "after clear the pool must create fresh children, not reuse the cleared ones",
            listOf("child3", "child4"),
            visible,
        )
        assertEquals("the fresh children must be re-added to the parent", 5, added.size)
    }
}
