//
//  H264Encoder.swift
//  BroadcastExtension
//
//  Hardware H.264 encoder using VideoToolbox
//

import Foundation
import VideoToolbox
import CoreMedia

protocol H264EncoderDelegate: AnyObject {
    func encoder(_ encoder: H264Encoder, didEncodePacket packet: Data, pts: Int64, isKeyframe: Bool)
    func encoder(_ encoder: H264Encoder, didEncodeSpsPps sps: Data, pps: Data)
}

class H264Encoder {
    
    weak var delegate: H264EncoderDelegate?
    
    private var session: VTCompressionSession?
    let width: Int32
    let height: Int32
    private let bitrate: Int32
    private var hasWrittenSpsPps = false
    
    init(width: Int32, height: Int32, bitrate: Int32) {
        self.width = width
        self.height = height
        self.bitrate = bitrate
    }
    
    func start() {
        // Create compression session
        let status = VTCompressionSessionCreate(
            allocator: kCFAllocatorDefault,
            width: width,
            height: height,
            codecType: kCMVideoCodecType_H264,
            encoderSpecification: nil,
            imageBufferAttributes: nil,
            compressedDataAllocator: nil,
            outputCallback: encoderOutputCallback,
            refcon: Unmanaged.passUnretained(self).toOpaque(),
            compressionSessionOut: &session
        )
        
        guard status == noErr, let session = session else {
            print("[ENCODER] Failed to create session: \(status)")
            return
        }
        
        // Configure encoder properties
        configureSession(session)
        
        // Prepare to encode
        VTCompressionSessionPrepareToEncodeFrames(session)
        
        print("[ENCODER] Started - \(width)x\(height) @ \(bitrate/1000)kbps")
    }
    
    private func configureSession(_ session: VTCompressionSession) {
        // Real-time encoding
        VTSessionSetProperty(session, key: kVTCompressionPropertyKey_RealTime, value: kCFBooleanTrue)
        
        // Bitrate
        VTSessionSetProperty(session, key: kVTCompressionPropertyKey_AverageBitRate, value: bitrate as CFNumber)
        
        // Keyframe interval (every 60 frames = ~1 second at 60fps)
        VTSessionSetProperty(session, key: kVTCompressionPropertyKey_MaxKeyFrameInterval, value: 60 as CFNumber)
        
        // Allow frame reordering (B-frames) - disable for lower latency
        VTSessionSetProperty(session, key: kVTCompressionPropertyKey_AllowFrameReordering, value: kCFBooleanFalse)
        
        // Profile: High for better quality
        VTSessionSetProperty(session, key: kVTCompressionPropertyKey_ProfileLevel, 
                            value: kVTProfileLevel_H264_High_AutoLevel)
        
        // Expected frame rate
        VTSessionSetProperty(session, key: kVTCompressionPropertyKey_ExpectedFrameRate, value: 60 as CFNumber)
        
        // Enforce bitrate limits (bytes per second, duration)
        let byteLimit = bitrate / 8
        let limitArgs: [Int] = [Int(byteLimit), 1] // limit bytes per 1 second
        VTSessionSetProperty(session, key: kVTCompressionPropertyKey_DataRateLimits, value: limitArgs as CFArray)
    }
    
    func stop() {
        guard let session = session else { return }
        
        VTCompressionSessionCompleteFrames(session, untilPresentationTimeStamp: .invalid)
        VTCompressionSessionInvalidate(session)
        self.session = nil
        
        print("[ENCODER] Stopped")
    }
    
    func encode(sampleBuffer: CMSampleBuffer) {
        guard let session = session,
              let imageBuffer = CMSampleBufferGetImageBuffer(sampleBuffer) else {
            return
        }
        
        let pts = CMSampleBufferGetPresentationTimeStamp(sampleBuffer)
        let duration = CMSampleBufferGetDuration(sampleBuffer)
        
        var flags: VTEncodeInfoFlags = []
        
        let status = VTCompressionSessionEncodeFrame(
            session,
            imageBuffer: imageBuffer,
            presentationTimeStamp: pts,
            duration: duration,
            frameProperties: nil,
            sourceFrameRefcon: nil,
            infoFlagsOut: &flags
        )
        
        if status != noErr {
            print("[ENCODER] Encode error: \(status)")
        }
    }
    
    // MARK: - Callback
    
    private let encoderOutputCallback: VTCompressionOutputCallback = { refcon, sourceFrameRefCon, status, flags, sampleBuffer in
        guard status == noErr,
              let sampleBuffer = sampleBuffer,
              let refcon = refcon else {
            return
        }
        
        let encoder = Unmanaged<H264Encoder>.fromOpaque(refcon).takeUnretainedValue()
        encoder.handleEncodedFrame(sampleBuffer: sampleBuffer)
    }
    
    private func handleEncodedFrame(sampleBuffer: CMSampleBuffer) {
        // Check if keyframe
        let isKeyframe = isKeyFrame(sampleBuffer)
        
        // Extract and send SPS/PPS on every keyframe to ensure new clients can decode
        if isKeyframe {
            extractSpsPps(from: sampleBuffer)
        }
        
        // Get encoded data
        guard let dataBuffer = CMSampleBufferGetDataBuffer(sampleBuffer) else { return }
        
        var length: Int = 0
        var dataPointer: UnsafeMutablePointer<Int8>?
        
        let status = CMBlockBufferGetDataPointer(
            dataBuffer,
            atOffset: 0,
            lengthAtOffsetOut: nil,
            totalLengthOut: &length,
            dataPointerOut: &dataPointer
        )
        
        guard status == noErr, let pointer = dataPointer else { return }
        
        // Convert AVCC to Annex-B format
        let data = convertToAnnexB(pointer: pointer, length: length)
        
        // Get PTS
        let pts = CMTimeGetSeconds(CMSampleBufferGetPresentationTimeStamp(sampleBuffer))
        let ptsUs = Int64(pts * 1_000_000)
        
        // Notify delegate
        delegate?.encoder(self, didEncodePacket: data, pts: ptsUs, isKeyframe: isKeyframe)
    }
    
    private func isKeyFrame(_ sampleBuffer: CMSampleBuffer) -> Bool {
        guard let attachments = CMSampleBufferGetSampleAttachmentsArray(sampleBuffer, createIfNecessary: false) as? [[CFString: Any]],
              let attachment = attachments.first else {
            return false
        }
        
        let notSync = attachment[kCMSampleAttachmentKey_NotSync] as? Bool ?? false
        return !notSync
    }
    
    private func extractSpsPps(from sampleBuffer: CMSampleBuffer) {
        guard let formatDesc = CMSampleBufferGetFormatDescription(sampleBuffer) else { return }
        
        // SPS
        var spsSize: Int = 0
        var spsCount: Int = 0
        var spsPointer: UnsafePointer<UInt8>?
        
        CMVideoFormatDescriptionGetH264ParameterSetAtIndex(
            formatDesc,
            parameterSetIndex: 0,
            parameterSetPointerOut: &spsPointer,
            parameterSetSizeOut: &spsSize,
            parameterSetCountOut: &spsCount,
            nalUnitHeaderLengthOut: nil
        )
        
        // PPS
        var ppsSize: Int = 0
        var ppsPointer: UnsafePointer<UInt8>?
        
        CMVideoFormatDescriptionGetH264ParameterSetAtIndex(
            formatDesc,
            parameterSetIndex: 1,
            parameterSetPointerOut: &ppsPointer,
            parameterSetSizeOut: &ppsSize,
            parameterSetCountOut: nil,
            nalUnitHeaderLengthOut: nil
        )
        
        if let spsPointer = spsPointer, let ppsPointer = ppsPointer {
            let sps = Data(bytes: spsPointer, count: spsSize)
            let pps = Data(bytes: ppsPointer, count: ppsSize)
            delegate?.encoder(self, didEncodeSpsPps: sps, pps: pps)
        }
    }
    
    /// Convert AVCC format to Annex-B (NAL start codes)
    private func convertToAnnexB(pointer: UnsafeMutablePointer<Int8>, length: Int) -> Data {
        var result = Data()
        let startCode: [UInt8] = [0x00, 0x00, 0x00, 0x01]
        
        var offset = 0
        while offset < length {
            // Read NAL unit length (4 bytes, big endian)
            var nalLength: UInt32 = 0
            memcpy(&nalLength, pointer.advanced(by: offset), 4)
            nalLength = CFSwapInt32BigToHost(nalLength)
            offset += 4
            
            // Add start code
            result.append(contentsOf: startCode)
            
            // Add NAL unit data
            result.append(Data(bytes: pointer.advanced(by: offset), count: Int(nalLength)))
            offset += Int(nalLength)
        }
        
        return result
    }
}
