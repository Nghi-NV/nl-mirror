package dev.nl.mirror.input

import android.os.SystemClock
import android.view.InputDevice
import android.view.KeyCharacterMap
import android.view.MotionEvent

/**
 * InputController handles raw event injection for mouse/touch and keyboard.
 * Uses reflection to access InputManager.injectInputEvent() for low-latency control.
 */
object InputController {
    @Volatile
    private var inputManager: Any? = null
    @Volatile
    private var injectInputEventMethod: java.lang.reflect.Method? = null
    @Volatile
    private var isInitialized = false

    private const val INJECT_INPUT_EVENT_MODE_ASYNC = 0

    // Track the downTime for each gesture (same downTime must be used for DOWN, MOVE, UP)
    private var lastDownTime: Long = 0L
    private var isPointerDown: Boolean = false

    /**
     * Initialize the InputController. Should be called during app startup.
     */
    @Synchronized
    fun init(): Boolean {
        if (isInitialized) return true
        
        repeat(5) {
            try {
                val imClass = Class.forName("android.hardware.input.InputManager")
                val getInstance = imClass.getMethod("getInstance")
                inputManager = getInstance.invoke(null)
                injectInputEventMethod = imClass.getMethod(
                    "injectInputEvent",
                    android.view.InputEvent::class.java,
                    Int::class.javaPrimitiveType
                )
                isInitialized = true
                    return true
            } catch (_: Exception) {
                Thread.sleep(100)
            }
        }
        return false
    }

    private fun ensureInitialized(): Boolean {
        if (!isInitialized) {
            init()
        }
        return isInitialized
    }

    /**
     * Injects a touch event at the specified coordinates.
     * Important: For a gesture sequence (DOWN->MOVE->UP), the downTime must be consistent.
     */
    fun injectTouch(action: Int, x: Float, y: Float, pointerId: Int = 0): Boolean {
        val now = SystemClock.uptimeMillis()
        
        // Track downTime properly
        val downTime = when (action) {
            MotionEvent.ACTION_DOWN -> {
                lastDownTime = now
                isPointerDown = true
                now
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                isPointerDown = false
                lastDownTime.takeIf { it > 0 } ?: now
            }
            else -> {
                // MOVE events use the original downTime
                lastDownTime.takeIf { it > 0 } ?: now
            }
        }
        
        val pointerProperties = arrayOf(MotionEvent.PointerProperties().apply {
            id = pointerId
            toolType = MotionEvent.TOOL_TYPE_FINGER
        })
        
        val pointerCoords = arrayOf(MotionEvent.PointerCoords().apply {
            this.x = x
            this.y = y
            pressure = if (action == MotionEvent.ACTION_UP) 0f else 1f
            size = 1f
        })

        val event = MotionEvent.obtain(
            downTime, now, action,
            1, pointerProperties, pointerCoords,
            0, 0, 1f, 1f,
            0, 0, InputDevice.SOURCE_TOUCHSCREEN, 0
        )

        val result = injectEvent(event)
        event.recycle() // Important: recycle MotionEvent to avoid memory leak
        return result
    }

    /**
     * Simulates a tap at the specified coordinates.
     */
    fun tap(x: Float, y: Float): Boolean {
        val downResult = injectTouch(MotionEvent.ACTION_DOWN, x, y)
        Thread.sleep(10)
        val upResult = injectTouch(MotionEvent.ACTION_UP, x, y)
        return downResult && upResult
    }

    /**
     * Simulates a swipe from (x1, y1) to (x2, y2).
     * Uses dynamic steps based on distance for smooth motion.
     */
    fun swipe(x1: Float, y1: Float, x2: Float, y2: Float, durationMs: Long = 300): Boolean {
        // Calculate distance and determine steps (more distance = more steps)
        val dx = x2 - x1
        val dy = y2 - y1
        val distance = kotlin.math.sqrt(dx * dx + dy * dy)
        
        // Dynamic steps: 1 step per 10 pixels, min 10, max 50
        val steps = (distance / 10).toInt().coerceIn(10, 50)
        val stepDuration = durationMs / steps
        
        // Use SystemClock for more accurate timing
        val startTime = android.os.SystemClock.uptimeMillis()
        
        injectTouch(MotionEvent.ACTION_DOWN, x1, y1)

        for (i in 1 until steps) {
            val ratio = i.toFloat() / steps
            // Apply ease-out curve for natural feel
            val easedRatio = 1 - (1 - ratio) * (1 - ratio)
            val x = x1 + dx * easedRatio
            val y = y1 + dy * easedRatio
            
            // Sleep only if needed to maintain timing
            val targetTime = startTime + (stepDuration * i)
            val sleepTime = targetTime - android.os.SystemClock.uptimeMillis()
            if (sleepTime > 0) {
                Thread.sleep(sleepTime)
            }
            
            injectTouch(MotionEvent.ACTION_MOVE, x, y)
        }

        return injectTouch(MotionEvent.ACTION_UP, x2, y2)
    }

    /**
     * Simulates a long press at the specified coordinates.
     */
    fun longPress(x: Float, y: Float, durationMs: Long = 500): Boolean {
        val downResult = injectTouch(MotionEvent.ACTION_DOWN, x, y)
        Thread.sleep(durationMs)
        val upResult = injectTouch(MotionEvent.ACTION_UP, x, y)
        return downResult && upResult
    }

    /**
     * Injects a key event with optional meta state (for modifiers like Ctrl, Alt).
     */
    fun injectKey(keyCode: Int, action: Int, metaState: Int = 0): Boolean {
        val now = SystemClock.uptimeMillis()
        val event = android.view.KeyEvent(
            now, now, action, keyCode, 0, metaState,
            KeyCharacterMap.VIRTUAL_KEYBOARD, 0, 0, InputDevice.SOURCE_KEYBOARD
        )
        return injectEvent(event)
    }

    /**
     * Simulates a key press (down + up).
     */
    fun pressKey(keyCode: Int): Boolean {
        val downResult = injectKey(keyCode, android.view.KeyEvent.ACTION_DOWN)
        val upResult = injectKey(keyCode, android.view.KeyEvent.ACTION_UP)
        return downResult && upResult
    }

    private fun injectEvent(event: android.view.InputEvent): Boolean {
        if (!ensureInitialized()) return false
        val manager = inputManager ?: return false
        val method = injectInputEventMethod ?: return false
        
        return try {
            method.invoke(manager, event, INJECT_INPUT_EVENT_MODE_ASYNC) as Boolean
        } catch (e: Exception) {
            e.printStackTrace()
            false
        }
    }

    /**
     * Injects text by generating KeyEvents for each character.
     * Uses KeyCharacterMap to convert characters to key codes.
     * Uses KeyComposition to decompose accented characters (é → ´e).
     */
    fun injectText(text: String): Boolean {
        return try {
            val keyCharacterMap = KeyCharacterMap.load(KeyCharacterMap.VIRTUAL_KEYBOARD)
            
            for (c in text) {
                // Try to decompose accented characters first
                val decomposed = KeyComposition.decompose(c)
                val chars = decomposed?.toCharArray() ?: charArrayOf(c)
                
                val events = keyCharacterMap.getEvents(chars)
                if (events != null) {
                    for (event in events) {
                        if (!injectEvent(event)) return false
                    }
                }
                // If events is null, the character cannot be typed (e.g., emoji)
                // Continue with the next character
            }
            true
        } catch (e: Exception) {
            e.printStackTrace()
            false
        }
    }
}
