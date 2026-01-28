package dev.nl.mirror.input

/**
 * PointersState - Manages state of multiple touch pointers
 * 
 * Ported from scrcpy's PointersState.java
 * 
 * Supports up to MAX_POINTERS (10) simultaneous touch points.
 * Maps client-provided IDs (which can be any long value) to local IDs (0-9)
 * for use with Android's MotionEvent.
 */
class PointersState {
    companion object {
        /** Maximum number of pointers supported by Android MotionEvent */
        const val MAX_POINTERS = 10
    }
    
    private val pointers = arrayOfNulls<Pointer>(MAX_POINTERS)
    
    /** Number of currently active (non-null, non-up) pointers */
    private var count = 0
    
    /**
     * Get or create a pointer for the given client ID.
     * @param clientId The ID provided by the client
     * @return The pointer, or null if MAX_POINTERS are already active
     */
    fun getOrCreate(clientId: Long): Pointer? {
        // First, try to find existing pointer with this clientId
        for (i in 0 until MAX_POINTERS) {
            val pointer = pointers[i]
            if (pointer != null && pointer.clientId == clientId) {
                return pointer
            }
        }
        
        // If not found, find a free slot
        for (i in 0 until MAX_POINTERS) {
            if (pointers[i] == null) {
                val pointer = Pointer(clientId, i)
                pointers[i] = pointer
                count++
                return pointer
            }
        }
        
        // No free slot available
        return null
    }
    
    /**
     * Get pointer by local ID (index).
     */
    fun get(localId: Int): Pointer? {
        return if (localId in 0 until MAX_POINTERS) pointers[localId] else null
    }
    
    /**
     * Find pointer by client ID.
     */
    fun findByClientId(clientId: Long): Pointer? {
        for (i in 0 until MAX_POINTERS) {
            val pointer = pointers[i]
            if (pointer != null && pointer.clientId == clientId) {
                return pointer
            }
        }
        return null
    }
    
    /**
     * Remove pointer by client ID.
     * Called after ACTION_UP for that pointer.
     */
    fun remove(clientId: Long) {
        for (i in 0 until MAX_POINTERS) {
            val pointer = pointers[i]
            if (pointer != null && pointer.clientId == clientId) {
                pointers[i] = null
                count--
                return
            }
        }
    }
    
    /**
     * Get the number of active pointers.
     */
    fun getCount(): Int = count
    
    /**
     * Get the index of a pointer within the active pointers list.
     * This is needed for MotionEvent's pointerIndex parameter.
     */
    fun getPointerIndex(localId: Int): Int {
        var index = 0
        for (i in 0 until localId) {
            if (pointers[i] != null) {
                index++
            }
        }
        return index
    }
    
    /**
     * Get all active pointers in order (for building MotionEvent arrays).
     */
    fun getActivePointers(): List<Pointer> {
        return pointers.filterNotNull()
    }
    
    /**
     * Check if all pointers are in UP state.
     */
    fun allPointersUp(): Boolean {
        for (i in 0 until MAX_POINTERS) {
            val pointer = pointers[i]
            if (pointer != null && !pointer.isUp) {
                return false
            }
        }
        return true
    }
    
    /**
     * Clear all pointers (reset state).
     */
    fun clear() {
        for (i in 0 until MAX_POINTERS) {
            pointers[i] = null
        }
        count = 0
    }
}
