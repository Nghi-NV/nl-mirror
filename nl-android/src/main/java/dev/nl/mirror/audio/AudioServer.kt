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
    private var currentCapture: AudioCapture? = null

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
        currentCapture?.stop()
        try { serverSocket?.close() } catch (_: Exception) {}
    }

    private fun handleClient(socket: Socket) {
        Thread({
            // Stop previous encoder if exists
            currentEncoder?.stop()
            currentCapture?.stop()
            
            var capture: AudioCapture? = null
            try {
                socket.tcpNoDelay = true
                socket.sendBufferSize = 64 * 1024
                socket.soTimeout = 5000 // 5 second timeout for detecting disconnect
                
                capture = AudioCapture()
                currentCapture = capture
                
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
                
                // Monitor connection by reading from socket
                val inputStream = socket.getInputStream()
                val buffer = ByteArray(1)
                while (!socket.isClosed && isRunning) {
                    try {
                        val bytesRead = inputStream.read(buffer)
                        if (bytesRead == -1) {
                            // Client disconnected
                            break
                        }
                    } catch (_: java.net.SocketTimeoutException) {
                        // Timeout is expected, continue checking
                        continue
                    } catch (_: Exception) {
                        break
                    }
                }
            } catch (_: Exception) {
            } finally {
                currentEncoder?.stop()
                currentEncoder = null
                capture?.stop()
                currentCapture = null
                try { socket.close() } catch (_: Exception) {}
            }
        }, "audio-client").start()
    }
}
