package dev.nl.mirror.control

import android.view.KeyEvent
import android.view.MotionEvent
import dev.nl.mirror.input.InputController
import dev.nl.mirror.input.TouchScaler
import dev.nl.mirror.util.PerformanceMonitor
import dev.nl.mirror.util.ViewHierarchyDumper
import java.io.DataOutputStream
import java.io.OutputStream

/**
 * BinaryCommandHandler - Processes binary control messages
 * 
 * Low-latency replacement for JSON-based CommandHandler.
 * Processes ControlMessage objects and returns binary responses.
 */
class BinaryCommandHandler(outputStream: OutputStream) {
    
    private val dos = DataOutputStream(outputStream.buffered())
    
    // Response types
    companion object {
        const val RESPONSE_OK = 0
        const val RESPONSE_ERROR = 1
        const val RESPONSE_CLIPBOARD = 2
        const val RESPONSE_HIERARCHY = 3
        const val RESPONSE_STATS = 4
    }
    
    /**
     * Handle a control message and send response if needed.
     */
    fun handle(msg: ControlMessage) {
        when (msg.type) {
            ControlMessage.TYPE_INJECT_KEYCODE -> {
                val action = if (msg.action == 0) KeyEvent.ACTION_DOWN else KeyEvent.ACTION_UP
                val success = InputController.injectKey(msg.keycode, action, msg.metaState)
                sendOkResponse(success)
            }
            
            ControlMessage.TYPE_INJECT_TEXT -> {
                msg.text?.let { text ->
                    Thread {
                        InputController.injectText(text)
                    }.start()
                    sendOkResponse(true)
                } ?: sendOkResponse(false)
            }
            
            ControlMessage.TYPE_INJECT_TOUCH_EVENT -> {
                val (x, y) = TouchScaler.transform(msg.x, msg.y)
                // Fire-and-forget for low latency (like scrcpy)
                InputController.injectTouch(msg.action, x, y, msg.pointerId.toInt())
                // No response sent for touch events to minimize latency
            }
            
            ControlMessage.TYPE_INJECT_SCROLL_EVENT -> {
                // TODO: Implement scroll event injection
                sendOkResponse(true)
            }
            
            ControlMessage.TYPE_BACK_OR_SCREEN_ON -> {
                val action = if (msg.action == 0) KeyEvent.ACTION_DOWN else KeyEvent.ACTION_UP
                val success = InputController.injectKey(KeyEvent.KEYCODE_BACK, action, 0)
                sendOkResponse(success)
            }
            
            ControlMessage.TYPE_GET_CLIPBOARD -> {
                val copy = msg.copyKey == ControlMessage.COPY_KEY_COPY
                val text = if (copy) {
                    ClipboardController.copyAndGetText() ?: ""
                } else {
                    ClipboardController.getText() ?: ""
                }
                sendClipboardResponse(text)
            }
            
            ControlMessage.TYPE_SET_CLIPBOARD -> {
                msg.text?.let { text ->
                    Thread {
                        ClipboardController.setTextAndPaste(text, msg.paste)
                    }.start()
                    sendOkResponse(true)
                } ?: sendOkResponse(false)
            }
            
            ControlMessage.TYPE_SET_DISPLAY_POWER -> {
                val mode = if (msg.on) 2 else 0 // 0=OFF, 2=NORMAL
                val success = dev.nl.mirror.video.DisplayControl.setPowerMode(mode)
                sendOkResponse(success)
            }
            
            // nl-mirror extensions
            ControlMessage.TYPE_TAP -> {
                val (x, y) = TouchScaler.transform(msg.x, msg.y)
                val success = InputController.tap(x, y)
                sendOkResponse(success)
            }
            
            ControlMessage.TYPE_SWIPE -> {
                val (x1, y1) = TouchScaler.transform(msg.x, msg.y)
                val (x2, y2) = TouchScaler.transform(msg.screenWidth.toFloat(), msg.screenHeight.toFloat())
                val success = InputController.swipe(x1, y1, x2, y2, msg.duration)
                sendOkResponse(success)
            }
            
            ControlMessage.TYPE_LONG_PRESS -> {
                val (x, y) = TouchScaler.transform(msg.x, msg.y)
                val success = InputController.longPress(x, y, msg.duration)
                sendOkResponse(success)
            }
            
            ControlMessage.TYPE_KEY -> {
                val success = InputController.pressKey(msg.keycode)
                sendOkResponse(success)
            }
            
            ControlMessage.TYPE_HIERARCHY -> {
                val hierarchy = ViewHierarchyDumper.dump().toString()
                sendHierarchyResponse(hierarchy)
            }
            
            ControlMessage.TYPE_STATS -> {
                val stats = PerformanceMonitor.getStats().toString()
                sendStatsResponse(stats)
            }
            
            ControlMessage.TYPE_START_MOCK_LOCATION -> {
                dev.nl.mirror.input.LocationController.startMocking()
                sendOkResponse(true)
            }
            
            ControlMessage.TYPE_STOP_MOCK_LOCATION -> {
                dev.nl.mirror.input.LocationController.stopMocking()
                sendOkResponse(true)
            }
            
            ControlMessage.TYPE_SET_LOCATION -> {
                dev.nl.mirror.input.LocationController.updateLocation(
                    msg.lat, msg.lon, msg.alt, msg.bearing, msg.speed
                )
                sendOkResponse(true)
            }
            
            else -> {
                sendOkResponse(false)
            }
        }
    }
    
    // ===== Response methods =====
    
    @Synchronized
    private fun sendOkResponse(success: Boolean) {
        dos.writeByte(RESPONSE_OK)
        dos.writeBoolean(success)
        dos.flush()
    }
    
    @Synchronized
    private fun sendClipboardResponse(text: String) {
        val bytes = text.toByteArray(Charsets.UTF_8)
        dos.writeByte(RESPONSE_CLIPBOARD)
        dos.writeInt(bytes.size)
        dos.write(bytes)
        dos.flush()
    }
    
    @Synchronized
    private fun sendHierarchyResponse(hierarchy: String) {
        val bytes = hierarchy.toByteArray(Charsets.UTF_8)
        dos.writeByte(RESPONSE_HIERARCHY)
        dos.writeInt(bytes.size)
        dos.write(bytes)
        dos.flush()
    }
    
    @Synchronized
    private fun sendStatsResponse(stats: String) {
        val bytes = stats.toByteArray(Charsets.UTF_8)
        dos.writeByte(RESPONSE_STATS)
        dos.writeInt(bytes.size)
        dos.write(bytes)
        dos.flush()
    }
}
