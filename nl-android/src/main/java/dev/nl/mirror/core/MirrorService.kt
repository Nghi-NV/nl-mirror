package dev.nl.mirror.core

import dev.nl.mirror.network.PacketWriter
import dev.nl.mirror.video.DisplayManager
import dev.nl.mirror.video.ScreenEncoder
import java.net.Socket

object MirrorService {
    private var currentSessionThread: Thread? = null
    private var isSessionRunning = false

    fun startSession(socket: Socket, bitrate: Int, maxResolution: Int) {
        stopSession()
        isSessionRunning = true
        currentSessionThread = Thread {
            val packetWriter = PacketWriter(socket.getOutputStream())
            packetWriter.start()
            val watcher = DisplayManager.RotationWatcher()
            watcher.start()
            var encoder: ScreenEncoder? = null

            try {
                while (isSessionRunning && !socket.isClosed && !Thread.interrupted()) {
                    val (w_phys, h_phys) = DisplayManager.getDisplaySize()
                    val rotation = watcher.getCurrentRotation()
                    val isLandscape = (rotation == 1 || rotation == 3)
                    val width = if (isLandscape) h_phys else w_phys
                    val height = if (isLandscape) w_phys else h_phys
                    val scale = if (width > maxResolution) maxResolution.toFloat() / width else 1.0f
                    val encW = (width * scale).toInt()
                    val encH = (height * scale).toInt()

                    encoder = ScreenEncoder(width, height, bitrate, packetWriter)
                    encoder.start()
                    watcher.resetChangeFlag()

                    while (isSessionRunning && !socket.isClosed) {
                        if (watcher.hasChanged()) break
                        Thread.sleep(100)
                    }
                    encoder.stop()
                    encoder = null
                }
            } catch (_: Exception) {
            } finally {
                encoder?.stop()
                watcher.stopWatcher()
                packetWriter.stop()
                try { socket.close() } catch (_: Exception) {}
            }
        }
        currentSessionThread?.start()
    }

    fun stopSession() {
        isSessionRunning = false
        currentSessionThread?.interrupt()
        try { currentSessionThread?.join(1000) } catch (_: Exception) {}
        currentSessionThread = null
    }
}
