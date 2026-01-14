package dev.nl.mirror.network

import java.net.ServerSocket
import java.util.concurrent.Executors

class CommandServer(private val port: Int) {
    private val executor = Executors.newCachedThreadPool()
    private var isRunning = false

    fun start() {
        Thread {
            try {
                val commandSocket = ServerSocket(port)
                isRunning = true
                while (isRunning) {
                    val client = commandSocket.accept()
                    executor.execute {
                        try {
                            val inputStream = client.getInputStream()
                            val outputStream = client.getOutputStream()
                            val reader = inputStream.bufferedReader()
                            while (!client.isClosed) {
                                val response = CommandHandler.handleCommand(reader)
                                outputStream.write((response + "\n").toByteArray())
                                outputStream.flush()
                            }
                        } catch (_: Exception) {
                        } finally {
                            try { client.close() } catch (_: Exception) {}
                        }
                    }
                }
            } catch (_: Exception) {}
        }.start()
    }
}
