package dev.nl.mirror.audio

import java.net.ServerSocket
import java.net.Socket

/**
 * TCP server for audio streaming on port 8890.
 * Each client connection gets its own AudioCapture + AudioEncoder pipeline.
 */
class AudioServer(private val port: Int = 8890) {
    private var serverSocket: ServerSocket? = null
    private var isRunning = false
    private var currentEncoder: AudioEncoder? = null

    fun start() {
        Thread({
            try {
                serverSocket = ServerSocket(port)
                isRunning = true
                
                while (isRunning) {
                    try {
                        val client = serverSocket?.accept() ?: break
                        handleClient(client)
                    } catch (e: Exception) {
                        if (!isRunning) break
                    }
                }
            } catch (_: Exception) {
            }
        }, "audio-server").start()
    }

    fun stop() {
        isRunning = false
        currentEncoder?.stop()
        try { serverSocket?.close() } catch (_: Exception) {}
    }

    private fun handleClient(socket: Socket) {
        Thread({
            // Stop previous encoder if exists
            currentEncoder?.stop()
            
            try {
                socket.tcpNoDelay = true
                socket.sendBufferSize = 64 * 1024
                
                val capture = AudioCapture()
                
                if (!capture.checkCompatibility()) {
                    socket.close()
                    return@Thread
                }
                
                val encoder = AudioEncoder(capture, socket.getOutputStream())
                currentEncoder = encoder
                
                if (!encoder.start()) {
                    socket.close()
                    return@Thread
                }
                
                // Keep connection alive until socket closes
                while (!socket.isClosed && isRunning) {
                    Thread.sleep(1000)
                }
            } catch (_: Exception) {
            } finally {
                currentEncoder?.stop()
                currentEncoder = null
                try { socket.close() } catch (_: Exception) {}
            }
        }, "audio-client").start()
    }
}
