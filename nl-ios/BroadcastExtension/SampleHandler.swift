//
//  SampleHandler.swift
//  BroadcastExtension
//
//  ReplayKit Broadcast Extension for screen capture
//

import ReplayKit

import Network

class StreamClient {
    private var connection: NWConnection?
    private let queue = DispatchQueue(label: "dev.nl.mirror.client")
    private var isConnected = false
    
    func connect() {
        if isConnected { return }
        
        let endpoint = NWEndpoint.hostPort(host: "127.0.0.1", port: 9998) // Internal IPC port
        connection = NWConnection(to: endpoint, using: .tcp)
        
        connection?.stateUpdateHandler = { [weak self] state in
            switch state {
            case .ready:
                print("[CLIENT] Connected to main app")
                self?.isConnected = true
            case .failed(let error):
                print("[CLIENT] Connection failed: \(error)")
                self?.isConnected = false
            case .cancelled:
                self?.isConnected = false
            case .waiting(let error):
                print("[CLIENT] Waiting: \(error)")
            default:
                break
            }
        }
        
        connection?.start(queue: queue)
    }
    
    func send(data: Data) {
        if !isConnected {
            connect()
            // Drop this packet or queue it? Dropping for realtime is better usually
            // But let's try to send anyway if it just became ready? 
            // NWConnection needs time. Just drop and wait for next frame.
            return
        }
        
        connection?.send(content: data, completion: .contentProcessed { [weak self] error in
            if let error = error {
                print("[CLIENT] Send error: \(error)")
                self?.handleError()
            }
        })
    }
    
    private func handleError() {
        isConnected = false
        connection?.cancel()
        connection = nil // Will reconnect on next send
    }
    
    func disconnect() {
        isConnected = false
        connection?.cancel()
        connection = nil
    }
}

class SampleHandler: RPBroadcastSampleHandler {
    
    private var encoder: H264Encoder?
    private var streamClient: StreamClient?
    private var frameCount: Int64 = 0
    
    override func broadcastStarted(withSetupInfo setupInfo: [String : NSObject]?) {
        print("[BROADCAST] Started")
        
        // Connect to main app
        streamClient = StreamClient()
        streamClient?.connect()
        
        // Initialize encoder is deferred to first frame arrive
        // encoder = H264Encoder(...)
    }
    
    override func broadcastPaused() {
        print("[BROADCAST] Paused")
    }
    
    override func broadcastResumed() {
        print("[BROADCAST] Resumed")
        // Re-establish connection immediately
        streamClient?.disconnect()
        streamClient?.connect()
        
        // Force encoder reset on next frame
        encoder?.stop()
        encoder = nil
    }
    
    override func broadcastFinished() {
        print("[BROADCAST] Finished")
        encoder?.stop()
        encoder = nil
        streamClient?.disconnect()
        streamClient = nil
    }
    
    override func processSampleBuffer(_ sampleBuffer: CMSampleBuffer, with sampleBufferType: RPSampleBufferType) {
        switch sampleBufferType {
        case .video:
            // Get frame dimensions
            guard let imageBuffer = CMSampleBufferGetImageBuffer(sampleBuffer) else { return }
            let width = Int32(CVPixelBufferGetWidth(imageBuffer))
            let height = Int32(CVPixelBufferGetHeight(imageBuffer))
            
            // Re-init encoder if dimensions changed or not initialized
            if encoder == nil || encoder?.width != width || encoder?.height != height {
                print("[BROADCAST] Resolution changed: \(width)x\(height)")
                encoder?.stop()
                // Use 15 Mbps for high quality LAN streaming
                encoder = H264Encoder(width: width, height: height, bitrate: 15_000_000)
                encoder?.delegate = self
                encoder?.start()
            }
            
            encoder?.encode(sampleBuffer: sampleBuffer)
            frameCount += 1
            if frameCount % 60 == 0 {
                print("[BROADCAST] Encoded \(frameCount) frames")
            }
        case .audioApp, .audioMic:
            break
        @unknown default:
            break
        }
    }
}

// MARK: - H264EncoderDelegate

extension SampleHandler: H264EncoderDelegate {
    func encoder(_ encoder: H264Encoder, didEncodePacket packet: Data, pts: Int64, isKeyframe: Bool) {
        // Send directly via socket
        streamClient?.send(data: packet)
    }
    
    func encoder(_ encoder: H264Encoder, didEncodeSpsPps sps: Data, pps: Data) {
        print("[BROADCAST] Sending SPS (\(sps.count)) / PPS (\(pps.count))")
        let startCode: [UInt8] = [0x00, 0x00, 0x00, 0x01]
        
        // Send SPS
        var spsData = Data(startCode)
        spsData.append(sps)
        streamClient?.send(data: spsData)
        
        // Send PPS
        var ppsData = Data(startCode)
        ppsData.append(pps)
        streamClient?.send(data: ppsData)
    }
}
