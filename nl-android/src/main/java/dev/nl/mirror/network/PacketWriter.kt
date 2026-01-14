package dev.nl.mirror.network

import java.io.OutputStream
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean

class PacketWriter(private val outputStream: OutputStream) {
    private val writeQueue = LinkedBlockingQueue<ByteArray>(100)
    private val isRunning = AtomicBoolean(false)
    private var writerThread: Thread? = null

    fun start() {
        if (isRunning.get()) return
        isRunning.set(true)
        writerThread = Thread {
            while (isRunning.get()) {
                try {
                    val data = writeQueue.poll(100, TimeUnit.MILLISECONDS) ?: continue
                    outputStream.write(data)
                    outputStream.flush()
                } catch (_: Exception) {
                    stop()
                    break
                }
            }
        }.apply { start() }
    }

    fun stop() {
        isRunning.set(false)
        writerThread?.interrupt()
        writerThread = null
    }

    fun queuePacket(data: ByteArray): Boolean {
        if (!isRunning.get()) return false
        return writeQueue.offer(data)
    }
}
