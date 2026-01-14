package dev.nl.mirror.network

import java.net.ServerSocket
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit

class CommandServer(private val port: Int) {
    private val executor = Executors.newCachedThreadPool()
    @Volatile
    private var isRunning = false
    private var serverSocket: ServerSocket? = null

    fun start() {
        Thread {
            try {
                serverSocket = ServerSocket(port)
                isRunning = true
                while (isRunning) {
                    try {
                        val client = serverSocket?.accept() ?: break
                        executor.execute {
                            try {
                                val inputStream = client.getInputStream()
                                val outputStream = client.getOutputStream()
                                val reader = inputStream.bufferedReader()
                                while (!client.isClosed && isRunning) {
                                    val response = CommandHandler.handleCommand(reader)
                                    outputStream.write((response + "\n").toByteArray())
                                    outputStream.flush()
                                }
                            } catch (_: Exception) {
                            } finally {
                                try { client.close() } catch (_: Exception) {}
                            }
                        }
                    } catch (_: Exception) {
                        if (!isRunning) break
                    }
                }
            } catch (_: Exception) {
            } finally {
                try { serverSocket?.close() } catch (_: Exception) {}
            }
        }.start()
    }

    fun stop() {
        isRunning = false
        try { serverSocket?.close() } catch (_: Exception) {}
        executor.shutdown()
        try {
            if (!executor.awaitTermination(2, TimeUnit.SECONDS)) {
                executor.shutdownNow()
            }
        } catch (_: Exception) {
            executor.shutdownNow()
        }
    }
}
