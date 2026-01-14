package dev.nl.mirror.video

import android.content.Context
import android.hardware.display.DisplayManager
import java.util.concurrent.TimeUnit

object DisplayManager {

    fun getDisplaySize(): Pair<Int, Int> {
        try {
            val process = Runtime.getRuntime().exec("wm size")
            val reader = java.io.BufferedReader(java.io.InputStreamReader(process.inputStream))
            val line = reader.readLine() // "Physical size: 1080x2400"
            if (line != null && line.contains(":")) {
                val parts = line.split(":")[1].trim().split("x")
                if (parts.size == 2) {
                    val w = parts[0].toInt()
                    val h = parts[1].toInt()
                    return Pair(w, h)
                }
            }
        } catch (e: Exception) {
            e.printStackTrace()
        }
        return Pair(720, 1280) // Fallback
    }

    class RotationWatcher : Thread() {
        private var displayManager: android.hardware.display.DisplayManager? = null
        private var looper: android.os.Looper? = null
        @Volatile private var rotation = 0
        @Volatile private var changed = false
        private val readyLatch = java.util.concurrent.CountDownLatch(1)

        override fun run() {
            android.os.Looper.prepare()
            looper = android.os.Looper.myLooper()
            
            try {
                // Get System Context via Reflection
                val activityThreadClass = Class.forName("android.app.ActivityThread")
                val systemMainMethod = activityThreadClass.getMethod("systemMain")
                val activityThread = systemMainMethod.invoke(null)
                val getSystemContextMethod = activityThreadClass.getMethod("getSystemContext")
                val context = getSystemContextMethod.invoke(activityThread) as Context
                
                displayManager = context.getSystemService(Context.DISPLAY_SERVICE) as android.hardware.display.DisplayManager
                
                // Initial rotation
                rotation = displayManager?.getDisplay(0)?.rotation ?: 0
                
                displayManager?.registerDisplayListener(object : android.hardware.display.DisplayManager.DisplayListener {
                    override fun onDisplayAdded(displayId: Int) {}
                    override fun onDisplayRemoved(displayId: Int) {}
                    override fun onDisplayChanged(displayId: Int) {
                        try {
                            val newRotation = displayManager?.getDisplay(0)?.rotation ?: 0
                            if (newRotation != rotation) {
                                rotation = newRotation
                                changed = true
                            }
                        } catch (e: Exception) {}
                    }
                }, null)
            } catch (e: Exception) {
                println("[WATCHER] Error initializing: ${e.message}")
            }
            
            readyLatch.countDown()
            android.os.Looper.loop()
        }
        
        fun stopWatcher() {
            looper?.quit()
        }

        fun getCurrentRotation(): Int {
            try { readyLatch.await(2, TimeUnit.SECONDS) } catch (e: Exception) {}
            return rotation
        }

        fun hasChanged(): Boolean {
            return changed
        }
        
        fun resetChangeFlag() {
            changed = false
        }
    }
}
