package dev.nl.mirror.network

import dev.nl.mirror.core.MirrorService
import java.net.ServerSocket
import java.net.Socket
import java.io.BufferedReader
import java.io.InputStreamReader

class SocketServer(private val port: Int) {
    private var serverSocket: ServerSocket? = null
    private var isRunning = false

    fun start() {
        try {
            serverSocket = ServerSocket(port)
            isRunning = true
            while (isRunning) {
                try {
                    val socket = serverSocket!!.accept()
                    handleClient(socket)
                } catch (_: Exception) {}
            }
        } catch (_: Exception) {}
    }

    private fun handleClient(socket: Socket) {
        Thread {
            try {
                socket.soTimeout = 0
                socket.keepAlive = true
                socket.tcpNoDelay = true
                socket.sendBufferSize = 64 * 1024

                var bitrate = 8_000_000
                var maxResolution = 1080
                try {
                    socket.soTimeout = 500
                    val reader = BufferedReader(InputStreamReader(socket.getInputStream()))
                    val line = reader.readLine()
                    if (line != null) {
                        val parts = line.split("&")
                        for (part in parts) {
                            val kv = part.split("=")
                            if (kv.size == 2) {
                                when(kv[0]) {
                                    "bitrate" -> bitrate = kv[1].toIntOrNull() ?: bitrate
                                    "max_size" -> maxResolution = kv[1].toIntOrNull() ?: maxResolution
                                }
                            }
                        }
                    }
                    socket.soTimeout = 0
                } catch (_: Exception) {
                    socket.soTimeout = 0
                }
                MirrorService.startSession(socket, bitrate, maxResolution)
            } catch (_: Exception) {
                try { socket.close() } catch (_: Exception) {}
            }
        }.start()
    }

    fun stop() {
        isRunning = false
        try { serverSocket?.close() } catch (_: Exception) {}
    }
}
