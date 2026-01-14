//
//  SharedBuffer.swift
//  Shared
//
//  IPC between Broadcast Extension and Main App using App Group
//

import Foundation

class SharedBuffer {
    
    static let shared = SharedBuffer()
    
    // App Group identifier - must match in both app and extension entitlements
    private let appGroupId = "group.com.nl.mirror"
    
    private var sharedDefaults: UserDefaults? {
        return UserDefaults(suiteName: appGroupId)
    }
    
    private init() {}
    
    // MARK: - Write (from Extension)
    
    func write(packet: Data, pts: Int64) {
        guard let defaults = sharedDefaults else { return }
        
        // Create packet with header
        var data = Data()
        
        // PTS (8 bytes)
        var ptsBE = pts.bigEndian
        data.append(Data(bytes: &ptsBE, count: 8))
        
        // Size (4 bytes)
        var sizeBE = Int32(packet.count).bigEndian
        data.append(Data(bytes: &sizeBE, count: 4))
        
        // Payload
        data.append(packet)
        
        // Write to shared container
        var packets = defaults.array(forKey: "video_packets") as? [Data] ?? []
        packets.append(data)
        
        // Keep only last 30 packets (buffer)
        if packets.count > 30 {
            packets.removeFirst(packets.count - 30)
        }
        
        defaults.set(packets, forKey: "video_packets")
    }
    
    func writeSpsPps(sps: Data, pps: Data) {
        guard let defaults = sharedDefaults else { return }
        defaults.set(sps, forKey: "video_sps")
        defaults.set(pps, forKey: "video_pps")
    }
    
    // MARK: - Read (from Main App)
    
    func readPackets() -> [Data] {
        guard let defaults = sharedDefaults else { return [] }
        let packets = defaults.array(forKey: "video_packets") as? [Data] ?? []
        defaults.removeObject(forKey: "video_packets")
        return packets
    }
    
    func readSpsPps() -> (sps: Data?, pps: Data?) {
        guard let defaults = sharedDefaults else { return (nil, nil) }
        let sps = defaults.data(forKey: "video_sps")
        let pps = defaults.data(forKey: "video_pps")
        return (sps, pps)
    }
    
    // MARK: - Clear
    
    func clear() {
        guard let defaults = sharedDefaults else { return }
        defaults.removeObject(forKey: "video_packets")
        defaults.removeObject(forKey: "video_sps")
        defaults.removeObject(forKey: "video_pps")
    }
}
