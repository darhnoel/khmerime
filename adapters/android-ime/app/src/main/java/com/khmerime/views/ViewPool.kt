package com.khmerime.views

// ViewPool
// ========
// Tracks a reusable set of children instead of removing/recreating views
// every render. Grows on demand and hides excess rather than removing them.
// Mirrors iOS's StripLabelPool.
//
// The Android wiring (creating a real view, attaching it to a parent,
// toggling visibility) is injected as plain lambdas rather than baked in
// directly against View/ViewGroup, so the grow/hide-excess policy can be
// unit-tested on the JVM without Robolectric.
class ViewPool<V>(
    private val createChild: () -> V,
    private val addChild: (V) -> Unit,
    private val setVisible: (V, Boolean) -> Unit,
    private val removeChild: (V) -> Unit = {},
) {
    private val children = mutableListOf<V>()

    // Ensures exactly `count` visible children exist, in order. Returns the
    // visible slice.
    fun sync(count: Int): List<V> {
        while (children.size < count) {
            val child = createChild()
            addChild(child)
            children.add(child)
        }
        children.forEachIndexed { i, child -> setVisible(child, i < count) }
        return children.take(count)
    }

    // Detaches every pooled child from its parent and drops the references.
    // Called when the input view is destroyed so this service-scoped pool does
    // not pin the old view hierarchy; the next sync() rebuilds against the new
    // input view.
    fun clear() {
        children.forEach(removeChild)
        children.clear()
    }
}
