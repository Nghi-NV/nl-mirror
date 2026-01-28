package dev.nl.mirror.network

import dev.nl.mirror.control.BinaryCommandHandler
import dev.nl.mirror.control.BinaryCommandReader
import java.net.ServerSocket
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit

/**
 * BinaryCommandServer - Server for binary control protocol
 * 
 * Runs on port 8891 (separate from JSON port 8889) for binary commands.
 * Provides lower latency than JSON-based CommandServer.
 */
class BinaryCommandServer(private val port: Int) {
    private val executor = Executors.newCachedThreadPool()
    @Volatile
    private var isRunning = false
    private var serverSocket: ServerSocket? = null

    fun start() {
        Thread {
            try {
                serverSocket = ServerSocket(port)
                isRunning = true
                println("[BINARY] Server started on port $port")
                
                while (isRunning) {
                    try {
                        val client = serverSocket?.accept() ?: break
                        client.tcpNoDelay = true
                        client.keepAlive = true
                        
                        executor.execute {
                            try {
                                val reader = BinaryCommandReader(client.getInputStream())
                                val handler = BinaryCommandHandler(client.getOutputStream())
                                
                                while (!client.isClosed && isRunning) {
                                    try {
                                        val msg = reader.read()
                                        handler.handle(msg)
                                    } catch (e: java.io.EOFException) {
                                        break // Client disconnected
                                    } catch (e: Exception) {
                                        println("[BINARY] Error handling message: ${e.message}")
                                        break
                                    }
                                }
                            } catch (e: Exception) {
                                println("[BINARY] Client error: ${e.message}")
                            } finally {
                                try { client.close() } catch (_: Exception) {}
                            }
                        }
                    } catch (e: Exception) {
                        if (!isRunning) break
                    }
                }
            } catch (e: Exception) {
                println("[BINARY] Server error: ${e.message}")
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
