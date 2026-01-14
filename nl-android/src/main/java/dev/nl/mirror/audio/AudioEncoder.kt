package dev.nl.mirror.audio

import android.media.MediaCodec
import java.io.OutputStream
import java.nio.ByteBuffer
import java.util.concurrent.atomic.AtomicBoolean

/**
 * Streams raw PCM audio from AudioCapture to output.
 * No encoding - maximum compatibility.
 */
class AudioEncoder(
    private val capture: AudioCapture,
    private val outputStream: OutputStream
) {
    private var inputThread: Thread? = null
    private val isRunning = AtomicBoolean(false)

    fun start(): Boolean {
        if (!capture.start()) {
            return false
        }
        isRunning.set(true)
        writeAudioHeader()
        
        inputThread = Thread({ streamLoop() }, "audio-stream").apply { start() }
        
        return true
    }

    fun stop() {
        isRunning.set(false)
        inputThread?.interrupt()
        try { inputThread?.join(1000) } catch (_: Exception) {}
        capture.stop()
    }

    private fun streamLoop() {
        val bufferInfo = MediaCodec.BufferInfo()
        val pcmBuffer = ByteBuffer.allocateDirect(AudioConfig.MAX_READ_SIZE)
        var packetCount = 0L
        
        while (isRunning.get()) {
            try {
                val bytesRead = capture.read(pcmBuffer, bufferInfo)
                if (bytesRead > 0) {
                    // Convert PCM i16 to bytes
                    val data = ByteArray(bytesRead)
                    pcmBuffer.position(0)
                    pcmBuffer.get(data, 0, bytesRead)
                    
                    // Send packet: [PTS(8)][Size(4)][Data(N)]
                    val packet = ByteBuffer.allocate(12 + data.size)
                    packet.putLong(bufferInfo.presentationTimeUs)
                    packet.putInt(data.size)
                    packet.put(data)
                    
                    synchronized(outputStream) {
                        outputStream.write(packet.array())
                        // Don't flush every packet - buffer for efficiency
                        if (packetCount % 10 == 0L) {
                            outputStream.flush()
                        }
                    }
                    
                    packetCount++
                }
            } catch (_: InterruptedException) {
                break
            } catch (e: Exception) {
                break
            }
        }
    }

    private fun writeAudioHeader() {
        // Header: "AUDIO" magic + sample rate (4) + channels (1) + codec type (1)
        val header = ByteBuffer.allocate(12)
        header.put("AUDIO\u0000".toByteArray()) // 6 bytes magic
        header.putInt(AudioConfig.SAMPLE_RATE)   // 4 bytes
        header.put(AudioConfig.CHANNELS.toByte()) // 1 byte
        header.put(0x00.toByte()) // 0 = RAW PCM i16
        
        synchronized(outputStream) {
            outputStream.write(header.array())
            outputStream.flush()
        }
    }
}
