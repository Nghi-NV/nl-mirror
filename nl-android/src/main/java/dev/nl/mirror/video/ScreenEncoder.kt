package dev.nl.mirror.video

import android.media.MediaCodec
import android.media.MediaCodecInfo
import android.media.MediaFormat
import android.os.Build
import android.view.Surface
import dev.nl.mirror.input.TouchScaler
import dev.nl.mirror.network.PacketWriter

class ScreenEncoder(
    private val width: Int,
    private val height: Int,
    private val bitrate: Int,
    private val packetWriter: PacketWriter
) {
    private var codec: MediaCodec? = null
    private var surface: Surface? = null
    private var isRunning = false

    fun start() {
        try {
            val encoderWidth = ((width + 15) / 16) * 16
            val encoderHeight = ((height + 15) / 16) * 16

            val format = MediaFormat.createVideoFormat("video/avc", encoderWidth, encoderHeight).apply {
                setInteger(MediaFormat.KEY_BIT_RATE, bitrate)
                setInteger(MediaFormat.KEY_FRAME_RATE, 60)
                setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, 1)
                setLong(MediaFormat.KEY_REPEAT_PREVIOUS_FRAME_AFTER, 100_000L)
                setInteger(MediaFormat.KEY_COLOR_FORMAT, MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface)
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                    setInteger(MediaFormat.KEY_PROFILE, MediaCodecInfo.CodecProfileLevel.AVCProfileConstrainedBaseline)
                }
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
                    setInteger("prepend-sps-pps-to-idr-frames", 1)
                }
            }

            codec = MediaCodec.createEncoderByType("video/avc")
            codec?.configure(format, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE)
            surface = codec?.createInputSurface()
            codec?.start()

            VirtualDisplayFactory.create("nl-mirror", width, height, encoderWidth, encoderHeight, surface!!)
            TouchScaler.configure(width, height, encoderWidth, encoderHeight)

            isRunning = true
            Thread { startEncodingLoop() }.start()
        } catch (e: Exception) {
            stop()
            throw e
        }
    }

    fun stop() {
        isRunning = false
        try {
            VirtualDisplayFactory.release()
            codec?.stop()
            codec?.release()
            surface?.release()
        } catch (_: Exception) {}
        codec = null
        surface = null
    }

    private fun startEncodingLoop() {
        val bufferInfo = MediaCodec.BufferInfo()
        val codec = this.codec ?: return

        while (isRunning) {
            try {
                val outputBufferId = codec.dequeueOutputBuffer(bufferInfo, 10000)
                if (outputBufferId >= 0) {
                    val outputBuffer = codec.getOutputBuffer(outputBufferId)
                    if (outputBuffer != null && bufferInfo.size > 0) {
                        outputBuffer.position(bufferInfo.offset)
                        outputBuffer.limit(bufferInfo.offset + bufferInfo.size)
                        val data = ByteArray(bufferInfo.size)
                        outputBuffer.get(data)

                        val packet = java.nio.ByteBuffer.allocate(12 + data.size)
                        packet.putLong(bufferInfo.presentationTimeUs)
                        packet.putInt(data.size)
                        packet.put(data)
                        packetWriter.queuePacket(packet.array())
                    }
                    codec.releaseOutputBuffer(outputBufferId, false)
                } else if (outputBufferId == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED) {
                    val format = codec.outputFormat
                    try {
                        val csd0 = format.getByteBuffer("csd-0")
                        val csd1 = format.getByteBuffer("csd-1")
                        if (csd0 != null) {
                            val spsData = ByteArray(csd0.remaining())
                            csd0.get(spsData)
                            val packet = java.nio.ByteBuffer.allocate(12 + spsData.size)
                            packet.putLong(0L)
                            packet.putInt(spsData.size)
                            packet.put(spsData)
                            packetWriter.queuePacket(packet.array())
                        }
                        if (csd1 != null) {
                            val ppsData = ByteArray(csd1.remaining())
                            csd1.get(ppsData)
                            val packet = java.nio.ByteBuffer.allocate(12 + ppsData.size)
                            packet.putLong(0L)
                            packet.putInt(ppsData.size)
                            packet.put(ppsData)
                            packetWriter.queuePacket(packet.array())
                        }
                    } catch (_: Exception) {}
                }
            } catch (_: Exception) {}
        }
    }
}
