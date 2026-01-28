package dev.nl.mirror.control

/**
 * ControlMessage - Represents a control message from the host
 * 
 * Binary protocol ported from scrcpy for low-latency control.
 * Uses compact binary encoding instead of JSON.
 */
data class ControlMessage(
    val type: Int,
    // Fields used by different message types
    val action: Int = 0,
    val keycode: Int = 0,
    val repeat: Int = 0,
    val metaState: Int = 0,
    val text: String? = null,
    val pointerId: Long = 0L,
    val x: Float = 0f,
    val y: Float = 0f,
    val screenWidth: Int = 0,
    val screenHeight: Int = 0,
    val pressure: Float = 0f,
    val actionButton: Int = 0,
    val buttons: Int = 0,
    val hScroll: Float = 0f,
    val vScroll: Float = 0f,
    val copyKey: Int = 0,
    val paste: Boolean = false,
    val on: Boolean = false,
    val mode: Int = 0,
    val lat: Double = 0.0,
    val lon: Double = 0.0,
    val alt: Double = 0.0,
    val bearing: Float = 0f,
    val speed: Float = 0f,
    val duration: Long = 0L
) {
    companion object {
        // Message types - matching scrcpy's protocol
        const val TYPE_INJECT_KEYCODE = 0
        const val TYPE_INJECT_TEXT = 1
        const val TYPE_INJECT_TOUCH_EVENT = 2
        const val TYPE_INJECT_SCROLL_EVENT = 3
        const val TYPE_BACK_OR_SCREEN_ON = 4
        const val TYPE_EXPAND_NOTIFICATION_PANEL = 5
        const val TYPE_EXPAND_SETTINGS_PANEL = 6
        const val TYPE_COLLAPSE_PANELS = 7
        const val TYPE_GET_CLIPBOARD = 8
        const val TYPE_SET_CLIPBOARD = 9
        const val TYPE_SET_DISPLAY_POWER = 10
        const val TYPE_ROTATE_DEVICE = 11
        
        // nl-mirror extensions
        const val TYPE_TAP = 20
        const val TYPE_SWIPE = 21
        const val TYPE_LONG_PRESS = 22
        const val TYPE_KEY = 23
        const val TYPE_HIERARCHY = 24
        const val TYPE_STATS = 25
        const val TYPE_START_MOCK_LOCATION = 26
        const val TYPE_STOP_MOCK_LOCATION = 27
        const val TYPE_SET_LOCATION = 28
        
        // Copy key values
        const val COPY_KEY_NONE = 0
        const val COPY_KEY_COPY = 1
        const val COPY_KEY_CUT = 2
        
        // Max sizes
        const val INJECT_TEXT_MAX_LENGTH = 300
        const val CLIPBOARD_TEXT_MAX_LENGTH = 256 * 1024 - 14
    }
}
