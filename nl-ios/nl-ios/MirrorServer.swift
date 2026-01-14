//
//  MirrorServer.swift
//  nl-ios
//
//  WebSocket/TCP server for streaming video to nl-host
//

import Foundation
import Network

class MirrorServer {
    private var listener: NWListener?
    private var internalListener: NWListener? // Listen for extension on localhost
    private var connections: [NWConnection] = []
    private var extensionConnection: NWConnection?
    private let queue = DispatchQueue(label: "dev.nl.mirror.server")
    
    static let shared = MirrorServer()
    
    func start(port: UInt16) {
        // 1. External Listener (for nl-host)
        do {
            let params = NWParameters.tcp
            params.allowLocalEndpointReuse = true
            
            listener = try NWListener(using: params, on: NWEndpoint.Port(rawValue: port)!)
            
            listener?.stateUpdateHandler = { state in
                switch state {
                case .ready:
                    print("[SERVER] External listening on port \(port)")
                case .failed(let error):
                    print("[SERVER] External failed: \(error)")
                default:
                    break
                }
            }
            
            listener?.newConnectionHandler = { [weak self] connection in
                self?.handleNewConnection(connection)
            }
            
            listener?.start(queue: queue)
            
        } catch {
            print("[SERVER] Failed to start external listener: \(error)")
        }
        
        // 2. Internal Listener (for Broadcast Extension)
        startInternalListener()
    }
    
    private func startInternalListener() {
        do {
            let params = NWParameters.tcp
            params.allowLocalEndpointReuse = true
            // Only bind to localhost for security
            params.requiredInterfaceType = .loopback
            
            internalListener = try NWListener(using: params, on: NWEndpoint.Port(rawValue: 9998)!)
            
            internalListener?.stateUpdateHandler = { state in
                switch state {
                case .ready:
                    print("[SERVER] Internal IPC listening on port 9998")
                case .failed(let error):
                    print("[SERVER] Internal IPC failed: \(error)")
                default:
                    break
                }
            }
            
            internalListener?.newConnectionHandler = { [weak self] connection in
                print("[SERVER] Extension connected!")
                self?.handleExtensionConnection(connection)
            }
            
            internalListener?.start(queue: queue)
            
        } catch {
            print("[SERVER] Failed to start internal listener: \(error)")
        }
    }
    
    func stop() {
        listener?.cancel()
        internalListener?.cancel()
        connections.forEach { $0.cancel() }
        connections.removeAll()
        extensionConnection?.cancel()
        extensionConnection = nil
    }
    
    private func handleNewConnection(_ connection: NWConnection) {
        print("[SERVER] New client connection from \(connection.endpoint)")
        
        connection.stateUpdateHandler = { [weak self] state in
            switch state {
            case .ready:
                print("[SERVER] Client ready")
                self?.connections.append(connection)
            case .failed(let error):
                print("[SERVER] Client failed: \(error)")
                self?.removeConnection(connection)
            case .cancelled:
                self?.removeConnection(connection)
            default:
                break
            }
        }
        
        connection.start(queue: queue)
    }
    
    private func handleExtensionConnection(_ connection: NWConnection) {
        extensionConnection = connection
        
        connection.stateUpdateHandler = { [weak self] state in
            switch state {
            case .ready:
                print("[SERVER] Extension stream ready")
                self?.readExtensionData(connection)
            case .failed(let error):
                print("[SERVER] Extension stream failed: \(error)")
            default:
                break
            }
        }
        
        connection.start(queue: queue)
    }
    
    private func readExtensionData(_ connection: NWConnection) {
        connection.receive(minimumIncompleteLength: 1, maximumLength: 65536) { [weak self] (data, _, isComplete, error) in
            if let data = data, !data.isEmpty {
                // Forward data to all external clients directly
                self?.broadcast(packet: data)
            }
            
            if isComplete {
                print("[SERVER] Extension disconnected")
            } else if error == nil {
                // Continue reading
                self?.readExtensionData(connection)
            } else {
                print("[SERVER] Extension read error: \(error!)")
            }
        }
    }
    
    private func removeConnection(_ connection: NWConnection) {
        connections.removeAll { $0 === connection }
    }
    
    /// Broadcast video packet to all connected clients
    func broadcast(packet: Data) {
        for connection in connections {
            connection.send(content: packet, completion: .contentProcessed { error in
                if let error = error {
                    print("[SERVER] Send error: \(error)")
                }
            })
        }
    }
    
    /// Send video packet with header
    func sendVideoPacket(data: Data, pts: Int64) {
        var packet = Data()
        
        // PTS (8 bytes, big endian)
        var ptsBE = pts.bigEndian
        packet.append(Data(bytes: &ptsBE, count: 8))
        
        // Size (4 bytes, big endian)
        var sizeBE = Int32(data.count).bigEndian
        packet.append(Data(bytes: &sizeBE, count: 4))
        
        // H.264 data
        packet.append(data)
        
        broadcast(packet: packet)
    }
    
    var hasConnections: Bool {
        return !connections.isEmpty
    }
}
