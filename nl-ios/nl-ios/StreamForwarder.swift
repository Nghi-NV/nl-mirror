//
//  StreamForwarder.swift
//  nl-ios
//
//  Forwards video packets from SharedBuffer to connected clients
//

import Foundation

class StreamForwarder {
    
    private var timer: Timer?
    private var isRunning = false
    private let server: MirrorServer
    
    init(server: MirrorServer) {
        self.server = server
    }
    
    func start() {
        guard !isRunning else { return }
        isRunning = true
        
        // Poll shared buffer and forward packets
        timer = Timer.scheduledTimer(withTimeInterval: 1.0/60.0, repeats: true) { [weak self] _ in
            self?.forwardPackets()
        }
        
        print("[FORWARDER] Started")
    }
    
    func stop() {
        isRunning = false
        timer?.invalidate()
        timer = nil
        print("[FORWARDER] Stopped")
    }
    
    private var packetCount = 0
    private var lastLogTime: Date = Date()
    
    private func forwardPackets() {
        // Check if we have clients
        if !server.hasConnections {
            return
        }
        
        // Read packets from shared buffer
        let packets = SharedBuffer.shared.readPackets()
        
        if !packets.isEmpty {
            packetCount += packets.count
            
            // Log every second
            if Date().timeIntervalSince(lastLogTime) >= 1.0 {
                print("[FORWARDER] Forwarded \(packetCount) packets")
                packetCount = 0
                lastLogTime = Date()
            }
        }
        
        for packet in packets {
            server.broadcast(packet: packet)
        }
    }
}
