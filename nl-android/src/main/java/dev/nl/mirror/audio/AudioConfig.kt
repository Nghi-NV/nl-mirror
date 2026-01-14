package dev.nl.mirror.audio

import android.media.AudioFormat
import android.media.MediaFormat

/**
 * Audio configuration constants for streaming.
 * Uses AAC-LC for maximum compatibility across Android devices.
 */
object AudioConfig {
    const val SAMPLE_RATE = 48000
    const val CHANNELS = 2
    val CHANNEL_CONFIG = AudioFormat.CHANNEL_IN_STEREO
    val CHANNEL_MASK = AudioFormat.CHANNEL_IN_LEFT or AudioFormat.CHANNEL_IN_RIGHT
    val ENCODING = AudioFormat.ENCODING_PCM_16BIT
    const val BYTES_PER_SAMPLE = 2
    
    // 1024 samples * 2 channels * 2 bytes = 4096 bytes per read
    const val MAX_READ_SIZE = 1024 * CHANNELS * BYTES_PER_SAMPLE
    
    // AAC-LC encoder settings (better compatibility than OPUS)
    const val AUDIO_BITRATE = 128000 // 128 kbps
    const val AUDIO_MIME_TYPE = MediaFormat.MIMETYPE_AUDIO_AAC // "audio/mp4a-latm"
    
    fun createAudioFormat(): AudioFormat {
        return AudioFormat.Builder()
            .setEncoding(ENCODING)
            .setSampleRate(SAMPLE_RATE)
            .setChannelMask(CHANNEL_CONFIG)
            .build()
    }
}
