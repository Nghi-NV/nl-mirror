package dev.nl.mirror.audio

import android.annotation.SuppressLint
import android.content.Context
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioManager
import android.media.AudioRecord
import android.media.MediaCodec
import android.media.MediaRecorder
import android.os.Build
import dev.nl.mirror.util.FakeContext
import dev.nl.mirror.util.Workarounds
import java.nio.ByteBuffer
import kotlin.math.abs

class AudioCapture {
    private var recorder: AudioRecord? = null
    private var audioPolicy: Any? = null
    private var nextPts: Long = 0
    private var readCount = 0L

    fun checkCompatibility(): Boolean {
        return Build.VERSION.SDK_INT >= Build.VERSION_CODES.R
    }

    @SuppressLint("MissingPermission")
    fun start(): Boolean {
        if (!checkCompatibility()) {
            return false
        }

        try {
            var started = false
            
            // Try AudioPolicy approach first (Android 13+)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                try {
                    recorder = createAudioRecordViaAudioPolicy()
                    recorder?.startRecording()
                    if (recorder?.recordingState == AudioRecord.RECORDSTATE_RECORDING) {
                        started = true
                    } else {
                        stop() // Clean up failed recorder
                    }
                } catch (_: Exception) {
                    stop()
                }
            }
            
            // Fallback to RemoteSubmix if AudioPolicy failed
            if (!started) {
                recorder = createAudioRecordRemoteSubmix()
                recorder?.startRecording()
                
                if (recorder?.recordingState == AudioRecord.RECORDSTATE_RECORDING) {
                    return true
                } else {
                    stop()
                    return false
                }
            }
            
            return true
        } catch (e: Exception) {
            stop()
            return false
        }
    }

    fun stop() {
        try {
            recorder?.stop()
            recorder?.release()
        } catch (_: Exception) {}
        recorder = null

        if (audioPolicy != null) {
            try {
                val context = FakeContext.get()
                val am = context.getSystemService(Context.AUDIO_SERVICE) as AudioManager
                val unregisterMethod = AudioManager::class.java.getMethod("unregisterAudioPolicyAsync", Class.forName("android.media.audiopolicy.AudioPolicy"))
                unregisterMethod.invoke(am, audioPolicy)
            } catch (_: Exception) {}
            audioPolicy = null
        }
    }

    fun read(buffer: ByteBuffer, bufferInfo: MediaCodec.BufferInfo): Int {
        val audioRecord = recorder ?: return -1
        buffer.clear()
        val bytesRead = audioRecord.read(buffer, AudioConfig.MAX_READ_SIZE)
        
        if (bytesRead > 0) {
            val pts = calculatePts(bytesRead)
            bufferInfo.set(0, bytesRead, pts, 0)
            buffer.position(0)
        }
        return bytesRead
    }




    private fun calculatePts(bytesRead: Int): Long {
        if (nextPts == 0L) {
            nextPts = System.nanoTime() / 1000
        }
        val pts = nextPts
        val durationUs = (bytesRead * 1000000L) / (2 * 2 * 48000)
        nextPts = pts + durationUs
        return pts
    }

    @SuppressLint("MissingPermission")
    private fun createAudioRecordRemoteSubmix(): AudioRecord {
        val builder = AudioRecord.Builder()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            builder.setContext(FakeContext.get())
        }
        builder.setAudioSource(MediaRecorder.AudioSource.REMOTE_SUBMIX)
        builder.setAudioFormat(AudioConfig.createAudioFormat())
        return builder.build()
    }

    @SuppressLint("PrivateApi", "DiscouragedPrivateApi")
    private fun createAudioRecordViaAudioPolicy(): AudioRecord {
        val context = Workarounds.getSystemContext() ?: FakeContext.get()
        val audioManager = context.getSystemService(Context.AUDIO_SERVICE) as AudioManager

        val mixRuleClass = Class.forName("android.media.audiopolicy.AudioMixingRule")
        val mixRuleBuilderClass = Class.forName("android.media.audiopolicy.AudioMixingRule\$Builder")
        val ruleBuilder = mixRuleBuilderClass.getConstructor().newInstance()

        val ruleMatchUsage = mixRuleClass.getField("RULE_MATCH_ATTRIBUTE_USAGE").getInt(null)
        val attr = AudioAttributes.Builder()
            .setUsage(AudioAttributes.USAGE_MEDIA)
            .build()
        
        mixRuleBuilderClass.getMethod("addMixRule", Int::class.javaPrimitiveType, Any::class.java)
            .invoke(ruleBuilder, ruleMatchUsage, attr)

        val mixingRule = mixRuleBuilderClass.getMethod("build").invoke(ruleBuilder)

        val mixClass = Class.forName("android.media.audiopolicy.AudioMix")
        val mixBuilderClass = Class.forName("android.media.audiopolicy.AudioMix\$Builder")
        val mixBuilder = mixBuilderClass.getConstructor(mixRuleClass).newInstance(mixingRule)

        mixBuilderClass.getMethod("setFormat", AudioFormat::class.java)
            .invoke(mixBuilder, AudioConfig.createAudioFormat())

        val routeFlag = mixClass.getField("ROUTE_FLAG_LOOP_BACK_RENDER").getInt(null)
        mixBuilderClass.getMethod("setRouteFlags", Int::class.javaPrimitiveType)
            .invoke(mixBuilder, routeFlag)

        val audioMix = mixBuilderClass.getMethod("build").invoke(mixBuilder)

        val policyClass = Class.forName("android.media.audiopolicy.AudioPolicy")
        val policyBuilderClass = Class.forName("android.media.audiopolicy.AudioPolicy\$Builder")
        val policyBuilder = policyBuilderClass.getConstructor(Context::class.java).newInstance(context)

        policyBuilderClass.getMethod("addMix", mixClass).invoke(policyBuilder, audioMix)
        val policy = policyBuilderClass.getMethod("build").invoke(policyBuilder)
        audioPolicy = policy

        val regMethod = AudioManager::class.java.getDeclaredMethod("registerAudioPolicyStatic", policyClass)
        regMethod.isAccessible = true
        val res = regMethod.invoke(null, policy) as Int
        if (res != 0) throw RuntimeException("registerAudioPolicyStatic failed: $res")

        val createSinkMethod = policyClass.getMethod("createAudioRecordSink", mixClass)
        return createSinkMethod.invoke(policy, audioMix) as AudioRecord
    }
}
