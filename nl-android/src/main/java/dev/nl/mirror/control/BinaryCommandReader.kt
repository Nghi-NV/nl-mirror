package dev.nl.mirror.control

import java.io.DataInputStream
import java.io.IOException
import java.io.InputStream
import java.nio.charset.StandardCharsets

/**
 * BinaryCommandReader - Reads binary control messages from the host
 * 
 * Ported from scrcpy's ControlMessageReader.java for low-latency control.
 * Binary protocol is more compact and faster to parse than JSON.
 * 
 * Wire format for each message type:
 * - TYPE_INJECT_KEYCODE: type(1) + action(1) + keycode(4) + repeat(4) + metaState(4) = 14 bytes
 * - TYPE_INJECT_TOUCH_EVENT: type(1) + action(1) + pointerId(8) + x(4) + y(4) + width(2) + height(2) + pressure(2) + actionButton(4) + buttons(4) = 32 bytes
 * - TYPE_INJECT_TEXT: type(1) + length(4) + text(n) = 5 + n bytes
 */
class BinaryCommandReader(inputStream: InputStream) {
    
    private val dis = DataInputStream(inputStream.buffered())
    
    /**
     * Read the next control message from the stream.
     * Blocks until a message is available.
     */
    @Throws(IOException::class)
    fun read(): ControlMessage {
        val type = dis.readUnsignedByte()
        
        return when (type) {
            ControlMessage.TYPE_INJECT_KEYCODE -> parseInjectKeycode()
            ControlMessage.TYPE_INJECT_TEXT -> parseInjectText()
            ControlMessage.TYPE_INJECT_TOUCH_EVENT -> parseInjectTouchEvent()
            ControlMessage.TYPE_INJECT_SCROLL_EVENT -> parseInjectScrollEvent()
            ControlMessage.TYPE_BACK_OR_SCREEN_ON -> parseBackOrScreenOn()
            ControlMessage.TYPE_GET_CLIPBOARD -> parseGetClipboard()
            ControlMessage.TYPE_SET_CLIPBOARD -> parseSetClipboard()
            ControlMessage.TYPE_SET_DISPLAY_POWER -> parseSetDisplayPower()
            ControlMessage.TYPE_EXPAND_NOTIFICATION_PANEL,
            ControlMessage.TYPE_EXPAND_SETTINGS_PANEL,
            ControlMessage.TYPE_COLLAPSE_PANELS,
            ControlMessage.TYPE_ROTATE_DEVICE -> ControlMessage(type = type)
            
            // nl-mirror extensions
            ControlMessage.TYPE_TAP -> parseTap()
            ControlMessage.TYPE_SWIPE -> parseSwipe()
            ControlMessage.TYPE_LONG_PRESS -> parseLongPress()
            ControlMessage.TYPE_KEY -> parseKey()
            ControlMessage.TYPE_HIERARCHY -> ControlMessage(type = type)
            ControlMessage.TYPE_STATS -> ControlMessage(type = type)
            ControlMessage.TYPE_START_MOCK_LOCATION -> ControlMessage(type = type)
            ControlMessage.TYPE_STOP_MOCK_LOCATION -> ControlMessage(type = type)
            ControlMessage.TYPE_SET_LOCATION -> parseSetLocation()
            
            else -> throw IOException("Unknown message type: $type")
        }
    }
    
    // ===== Parsing methods =====
    
    private fun parseInjectKeycode(): ControlMessage {
        val action = dis.readUnsignedByte()
        val keycode = dis.readInt()
        val repeat = dis.readInt()
        val metaState = dis.readInt()
        return ControlMessage(
            type = ControlMessage.TYPE_INJECT_KEYCODE,
            action = action,
            keycode = keycode,
            repeat = repeat,
            metaState = metaState
        )
    }
    
    private fun parseInjectText(): ControlMessage {
        val text = parseString()
        return ControlMessage(
            type = ControlMessage.TYPE_INJECT_TEXT,
            text = text
        )
    }
    
    private fun parseInjectTouchEvent(): ControlMessage {
        val action = dis.readUnsignedByte()
        val pointerId = dis.readLong()
        val x = dis.readInt()
        val y = dis.readInt()
        val screenWidth = dis.readUnsignedShort()
        val screenHeight = dis.readUnsignedShort()
        val pressure = u16FixedPointToFloat(dis.readShort())
        val actionButton = dis.readInt()
        val buttons = dis.readInt()
        return ControlMessage(
            type = ControlMessage.TYPE_INJECT_TOUCH_EVENT,
            action = action,
            pointerId = pointerId,
            x = x.toFloat(),
            y = y.toFloat(),
            screenWidth = screenWidth,
            screenHeight = screenHeight,
            pressure = pressure,
            actionButton = actionButton,
            buttons = buttons
        )
    }
    
    private fun parseInjectScrollEvent(): ControlMessage {
        val x = dis.readInt()
        val y = dis.readInt()
        val screenWidth = dis.readUnsignedShort()
        val screenHeight = dis.readUnsignedShort()
        // Scroll values are fixed-point, range [-16, 16]
        val hScroll = i16FixedPointToFloat(dis.readShort()) * 16
        val vScroll = i16FixedPointToFloat(dis.readShort()) * 16
        val buttons = dis.readInt()
        return ControlMessage(
            type = ControlMessage.TYPE_INJECT_SCROLL_EVENT,
            x = x.toFloat(),
            y = y.toFloat(),
            screenWidth = screenWidth,
            screenHeight = screenHeight,
            hScroll = hScroll,
            vScroll = vScroll,
            buttons = buttons
        )
    }
    
    private fun parseBackOrScreenOn(): ControlMessage {
        val action = dis.readUnsignedByte()
        return ControlMessage(
            type = ControlMessage.TYPE_BACK_OR_SCREEN_ON,
            action = action
        )
    }
    
    private fun parseGetClipboard(): ControlMessage {
        val copyKey = dis.readUnsignedByte()
        return ControlMessage(
            type = ControlMessage.TYPE_GET_CLIPBOARD,
            copyKey = copyKey
        )
    }
    
    private fun parseSetClipboard(): ControlMessage {
        dis.readLong() // sequence (unused)
        val paste = dis.readByte() != 0.toByte()
        val text = parseString()
        return ControlMessage(
            type = ControlMessage.TYPE_SET_CLIPBOARD,
            text = text,
            paste = paste
        )
    }
    
    private fun parseSetDisplayPower(): ControlMessage {
        val on = dis.readBoolean()
        return ControlMessage(
            type = ControlMessage.TYPE_SET_DISPLAY_POWER,
            on = on
        )
    }
    
    // ===== nl-mirror extensions =====
    
    private fun parseTap(): ControlMessage {
        val x = dis.readFloat()
        val y = dis.readFloat()
        return ControlMessage(
            type = ControlMessage.TYPE_TAP,
            x = x,
            y = y
        )
    }
    
    private fun parseSwipe(): ControlMessage {
        val x1 = dis.readFloat()
        val y1 = dis.readFloat()
        val x2 = dis.readFloat()
        val y2 = dis.readFloat()
        val duration = dis.readLong()
        return ControlMessage(
            type = ControlMessage.TYPE_SWIPE,
            x = x1,
            y = y1,
            screenWidth = x2.toInt(), // Reusing fields for x2, y2
            screenHeight = y2.toInt(),
            duration = duration
        )
    }
    
    private fun parseLongPress(): ControlMessage {
        val x = dis.readFloat()
        val y = dis.readFloat()
        val duration = dis.readLong()
        return ControlMessage(
            type = ControlMessage.TYPE_LONG_PRESS,
            x = x,
            y = y,
            duration = duration
        )
    }
    
    private fun parseKey(): ControlMessage {
        val keycode = dis.readInt()
        return ControlMessage(
            type = ControlMessage.TYPE_KEY,
            keycode = keycode
        )
    }
    
    private fun parseSetLocation(): ControlMessage {
        val lat = dis.readDouble()
        val lon = dis.readDouble()
        val alt = dis.readDouble()
        val bearing = dis.readFloat()
        val speed = dis.readFloat()
        return ControlMessage(
            type = ControlMessage.TYPE_SET_LOCATION,
            lat = lat,
            lon = lon,
            alt = alt,
            bearing = bearing,
            speed = speed
        )
    }
    
    // ===== Utility methods =====
    
    private fun parseString(): String {
        val length = dis.readInt()
        if (length > ControlMessage.CLIPBOARD_TEXT_MAX_LENGTH) {
            throw IOException("String too long: $length")
        }
        val data = ByteArray(length)
        dis.readFully(data)
        return String(data, StandardCharsets.UTF_8)
    }
    
    /**
     * Convert unsigned 16-bit fixed-point to float (range 0.0 - 1.0)
     */
    private fun u16FixedPointToFloat(value: Short): Float {
        val unsigned = value.toInt() and 0xFFFF
        return unsigned / 65535f
    }
    
    /**
     * Convert signed 16-bit fixed-point to float (range -1.0 - 1.0)
     */
    private fun i16FixedPointToFloat(value: Short): Float {
        return value / 32767f
    }
}
