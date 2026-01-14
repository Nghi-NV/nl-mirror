package dev.nl.mirror.video

import android.graphics.Rect
import android.hardware.display.DisplayManager
import android.hardware.display.VirtualDisplay
import android.os.Build
import android.os.IBinder
import android.view.Surface
import dev.nl.mirror.util.FakeContext
import java.lang.reflect.Method

object VirtualDisplayFactory {
    private var displayToken: IBinder? = null
    private var virtualDisplay: VirtualDisplay? = null

    fun release() {
        destroyDisplayToken()
        virtualDisplay?.release()
        virtualDisplay = null
    }

    private fun destroyDisplayToken() {
        val token = displayToken ?: return
        try {
            val scClass = Class.forName("android.view.SurfaceControl")
            val method = scClass.getMethod("destroyDisplay", IBinder::class.java)
            method.invoke(null, token)
        } catch (_: Exception) {}
        displayToken = null
    }

    fun create(name: String, deviceWidth: Int, deviceHeight: Int, encoderWidth: Int, encoderHeight: Int, surface: Surface) {
        // Try DisplayManager mirroring approach first (scrcpy's method)
        try {
            createMirrorDisplay(name, encoderWidth, encoderHeight, surface)
            return
        } catch (_: Exception) {}
        
        // Fallback to SurfaceControl approach (legacy)
        try {
            setupVirtualDisplayWithSC(name, deviceWidth, deviceHeight, encoderWidth, encoderHeight, surface)
        } catch (e: Exception) {
            throw RuntimeException("Could not create virtual display - all methods failed", e)
        }
    }
    
    private fun createMirrorDisplay(name: String, width: Int, height: Int, surface: Surface) {
        FakeContext.get()
        val dmClass = DisplayManager::class.java
        val mirrorMethod: Method = dmClass.getMethod(
            "createVirtualDisplay",
            String::class.java, Int::class.javaPrimitiveType, Int::class.javaPrimitiveType,
            Int::class.javaPrimitiveType, Surface::class.java
        )
        mirrorMethod.isAccessible = true
        virtualDisplay = mirrorMethod.invoke(null, name, width, height, 0, surface) as VirtualDisplay?
        if (virtualDisplay == null) throw RuntimeException("createVirtualDisplay returned null")
    }

    private fun setupVirtualDisplayWithSC(name: String, deviceWidth: Int, deviceHeight: Int, encoderWidth: Int, encoderHeight: Int, surface: Surface) {
        val scClass = Class.forName("android.view.SurfaceControl")
        val secure = Build.VERSION.SDK_INT < 30 || (Build.VERSION.SDK_INT == 30 && Build.VERSION.CODENAME != "S")
        val createDisplayMethod = scClass.getMethod("createDisplay", String::class.java, Boolean::class.javaPrimitiveType)
        val token = createDisplayMethod.invoke(null, name, secure) as IBinder
        this.displayToken = token

        val captureRect = Rect(0, 0, deviceWidth, deviceHeight)
        val displayRect = Rect(0, 0, encoderWidth, encoderHeight)

        val openTransactionMethod = scClass.getMethod("openTransaction")
        openTransactionMethod.invoke(null)
        try {
            scClass.getMethod("setDisplaySurface", IBinder::class.java, Surface::class.java).invoke(null, token, surface)
            scClass.getMethod("setDisplayProjection", IBinder::class.java, Int::class.javaPrimitiveType, Rect::class.java, Rect::class.java).invoke(null, token, 0, captureRect, displayRect)
            scClass.getMethod("setDisplayLayerStack", IBinder::class.java, Int::class.javaPrimitiveType).invoke(null, token, 0)
        } finally {
            scClass.getMethod("closeTransaction").invoke(null)
        }
    }
}
