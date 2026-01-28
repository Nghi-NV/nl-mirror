package dev.nl.mirror.core

import dev.nl.mirror.audio.AudioServer
import dev.nl.mirror.input.InputController
import dev.nl.mirror.network.BinaryCommandServer
import dev.nl.mirror.network.CommandServer
import dev.nl.mirror.network.SocketServer
import android.os.Build
import org.lsposed.hiddenapibypass.HiddenApiBypass

object App {
    @JvmStatic
    fun main(args: Array<String>) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            HiddenApiBypass.addHiddenApiExemptions("L")
        }
        
        // Initialize Looper and Workarounds on the startup thread
        // to avoid crashes when FakeContext is first accessed from a background thread.
        try {
            if (android.os.Looper.myLooper() == null) {
                android.os.Looper.prepareMainLooper()
            }
        } catch (_: Exception) {}
        
        dev.nl.mirror.util.Workarounds.apply()
        dev.nl.mirror.util.FakeContext.get()
        
        // Initialize InputController early to prevent race conditions
        InputController.init()
        
        try {
            // Video Server (port 8888)
            val videoServer = SocketServer(8888)
            Thread { videoServer.start() }.start()
            
            // Command Server - JSON (port 8889) - for backward compatibility
            val commandServer = CommandServer(8889)
            commandServer.start()
            
            // Binary Command Server (port 8891) - low-latency binary protocol
            val binaryServer = BinaryCommandServer(8891)
            binaryServer.start()
            
            // Audio Server (port 8890) - Android 11+ only
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                val audioServer = AudioServer(8890)
                audioServer.start()
            }
            
            Thread.currentThread().join()
        } catch (e: Exception) {
            System.exit(1)
        }
    }
}
