package dev.nl.mirror.input

import android.graphics.PointF

/**
 * Pointer - Represents a single touch pointer
 * 
 * Ported from scrcpy's Pointer.java
 * 
 * Each pointer has:
 * - clientId: the ID provided by the client (host)
 * - localId: the local ID used for MotionEvent (0-9)
 * - point: current position
 * - pressure: touch pressure (0-1)
 * - isUp: whether the pointer is currently in UP state
 */
class Pointer(
    /** ID provided by the client, can be any long value */
    var clientId: Long,
    /** Local ID for MotionEvent, must be 0-9 */
    var localId: Int
) {
    /** Current position of the pointer */
    val point: PointF = PointF()
    
    /** Touch pressure (0.0 - 1.0) */
    var pressure: Float = 1.0f
    
    /** Whether this pointer is in UP state */
    var isUp: Boolean = false
    
    /**
     * Set position and pressure for this pointer
     */
    fun setUp(x: Float, y: Float, pressure: Float) {
        this.point.set(x, y)
        this.pressure = pressure
        this.isUp = false
    }
    
    /**
     * Mark this pointer as UP
     */
    fun setAsUp() {
        this.isUp = true
        this.pressure = 0f
    }
}
