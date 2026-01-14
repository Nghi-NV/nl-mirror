
//
//  ViewController.swift
//  nl-ios
//
//  Created by User on 2026/01/05.
//

import UIKit
import ReplayKit
import AVFoundation
import Network

// MARK: - NetworkUtils Logic (Embedded)
// Putting here directly to avoid modifying pbxproj
func getDeviceIPAddress() -> String? {
    var address: String?
    var ifaddr: UnsafeMutablePointer<ifaddrs>?
    
    if getifaddrs(&ifaddr) == 0 {
        var ptr = ifaddr
        while ptr != nil {
            defer { ptr = ptr?.pointee.ifa_next }
            guard let interface = ptr?.pointee else { continue }
            let addrFamily = interface.ifa_addr.pointee.sa_family
            if addrFamily == UInt8(AF_INET) || addrFamily == UInt8(AF_INET6) {
                let name = String(cString: interface.ifa_name)
                if name == "en0" {
                    var hostname = [CChar](repeating: 0, count: Int(NI_MAXHOST))
                    getnameinfo(interface.ifa_addr, socklen_t(interface.ifa_addr.pointee.sa_len),
                                &hostname, socklen_t(hostname.count),
                                nil, socklen_t(0), NI_NUMERICHOST)
                    address = String(cString: hostname)
                    if addrFamily == UInt8(AF_INET) { break }
                }
            }
        }
        freeifaddrs(ifaddr)
    }
    return address
}

// MARK: - QRScannerViewController
class QRScannerViewController: UIViewController, AVCaptureMetadataOutputObjectsDelegate {

    var captureSession: AVCaptureSession!
    var previewLayer: AVCaptureVideoPreviewLayer!
    
    override func viewDidLoad() {
        super.viewDidLoad()
        
        view.backgroundColor = UIColor.black
        captureSession = AVCaptureSession()
        
        guard let videoCaptureDevice = AVCaptureDevice.default(for: .video) else { return }
        let videoInput: AVCaptureDeviceInput
        
        do {
            videoInput = try AVCaptureDeviceInput(device: videoCaptureDevice)
        } catch {
            return
        }
        
        if (captureSession.canAddInput(videoInput)) {
            captureSession.addInput(videoInput)
        } else {
            failed()
            return
        }
        
        let metadataOutput = AVCaptureMetadataOutput()
        
        if (captureSession.canAddOutput(metadataOutput)) {
            captureSession.addOutput(metadataOutput)
            
            metadataOutput.setMetadataObjectsDelegate(self, queue: DispatchQueue.main)
            metadataOutput.metadataObjectTypes = [.qr]
        } else {
            failed()
            return
        }
        
        previewLayer = AVCaptureVideoPreviewLayer(session: captureSession)
        previewLayer.frame = view.layer.bounds
        previewLayer.videoGravity = .resizeAspectFill
        view.layer.addSublayer(previewLayer)
        
        captureSession.startRunning()
        
        // Add Close Button
        let closeButton = UIButton(frame: CGRect(x: 20, y: 50, width: 80, height: 40))
        closeButton.setTitle("Close", for: .normal)
        closeButton.setTitleColor(.white, for: .normal)
        closeButton.addTarget(self, action: #selector(didTapClose), for: .touchUpInside)
        view.addSubview(closeButton)
        view.bringSubviewToFront(closeButton)
    }
    
    @objc func didTapClose() {
        dismiss(animated: true)
    }
    
    func failed() {
        let ac = UIAlertController(title: "Scanning not supported", message: "Your device does not support scanning a code from an item. Please use a device with a camera.", preferredStyle: .alert)
        ac.addAction(UIAlertAction(title: "OK", style: .default))
        present(ac, animated: true)
        captureSession = nil
    }
    
    override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)
        if (captureSession?.isRunning == false) {
            captureSession.startRunning()
        }
    }
    
    override func viewWillDisappear(_ animated: Bool) {
        super.viewWillDisappear(animated)
        if (captureSession?.isRunning == true) {
            captureSession.stopRunning()
        }
    }
    
    func metadataOutput(_ output: AVCaptureMetadataOutput, didOutput metadataObjects: [AVMetadataObject], from connection: AVCaptureConnection) {
        captureSession.stopRunning()
        
        if let metadataObject = metadataObjects.first {
            guard let readableObject = metadataObject as? AVMetadataMachineReadableCodeObject else { return }
            guard let stringValue = readableObject.stringValue else { return }
            AudioServicesPlaySystemSound(SystemSoundID(kSystemSoundID_Vibrate))
            found(code: stringValue)
        }
    }
    
    func found(code: String) {
        print("[QR] Found code: \(code)")
        // Assume Code is URL: http://<HOST>:8000/pair
        // We ensure we have Device IP
        guard let url = URL(string: code), let deviceIp = getDeviceIPAddress() else {
            let ac = UIAlertController(title: "Error", message: "Invalid QR or No WiFi IP", preferredStyle: .alert)
            ac.addAction(UIAlertAction(title: "OK", style: .default, handler: { _ in self.captureSession.startRunning() }))
            present(ac, animated: true)
            return
        }
        
        // Prepare Request
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        
        let body: [String: String] = ["device_ip": deviceIp]
        request.httpBody = try? JSONSerialization.data(withJSONObject: body)
        
        let task = URLSession.shared.dataTask(with: request) { data, response, error in
            DispatchQueue.main.async {
                if let error = error {
                    let ac = UIAlertController(title: "Connection Failed", message: error.localizedDescription, preferredStyle: .alert)
                    ac.addAction(UIAlertAction(title: "OK", style: .default, handler: { _ in self.captureSession.startRunning() }))
                    self.present(ac, animated: true)
                } else {
                    // Success!
                    let ac = UIAlertController(title: "Paired!", message: "Host received IP. Check your computer.", preferredStyle: .alert)
                    ac.addAction(UIAlertAction(title: "Done", style: .default, handler: { _ in self.dismiss(animated: true) }))
                    self.present(ac, animated: true)
                }
            }
        }
        task.resume()
    }
    
    override var prefersStatusBarHidden: Bool {
        return true
    }
    
    override var supportedInterfaceOrientations: UIInterfaceOrientationMask {
        return .portrait
    }
}


// MARK: - ViewController
class ViewController: UIViewController {
    
    private let statusLabel = UILabel()
    private let ipLabel = UILabel()
    private let portLabel = UILabel()
    private let startButton = UIButton(type: .system)
    private let stopButton = UIButton(type: .system)
    private let scanQRButton = UIButton(type: .system)
    
    private var mirrorServer: MirrorServer?
    private let serverPort: UInt16 = 9999
    
    override func viewDidLoad() {
        super.viewDidLoad()
        setupUI()
        startServer()
        
        // Init IP display
        ipLabel.text = "IP: \(getDeviceIPAddress() ?? "Unknown")"
    }
    
    private func setupUI() {
        view.backgroundColor = .systemBackground
        
        // Title
        let titleLabel = UILabel()
        titleLabel.text = "NL-iOS Mirror"
        titleLabel.font = .systemFont(ofSize: 32, weight: .bold)
        titleLabel.textAlignment = .center
        
        // Status
        statusLabel.text = "Ready to broadcast"
        statusLabel.textAlignment = .center
        statusLabel.textColor = .secondaryLabel
        
        // IP Address
        ipLabel.textAlignment = .center
        ipLabel.font = .monospacedSystemFont(ofSize: 18, weight: .medium)
        
        // Port
        portLabel.text = "Port: \(serverPort)"
        portLabel.textAlignment = .center
        portLabel.font = .monospacedSystemFont(ofSize: 18, weight: .medium)
        
        // Start Button
        startButton.setTitle("Start Broadcast", for: .normal)
        startButton.titleLabel?.font = .systemFont(ofSize: 20, weight: .semibold)
        startButton.backgroundColor = .systemBlue
        startButton.setTitleColor(.white, for: .normal)
        startButton.layer.cornerRadius = 12
        startButton.addTarget(self, action: #selector(startBroadcast), for: .touchUpInside)
        
        // Stop Button
        stopButton.setTitle("Stop Broadcast", for: .normal)
        stopButton.titleLabel?.font = .systemFont(ofSize: 20, weight: .semibold)
        stopButton.backgroundColor = .systemRed
        stopButton.setTitleColor(.white, for: .normal)
        stopButton.layer.cornerRadius = 12
        stopButton.addTarget(self, action: #selector(stopBroadcast), for: .touchUpInside)
        stopButton.isHidden = true
        
        // Scan QR Button
        scanQRButton.setTitle("Scan QR to Pair", for: .normal)
        scanQRButton.titleLabel?.font = .systemFont(ofSize: 18, weight: .medium)
        scanQRButton.setImage(UIImage(systemName: "qrcode.viewfinder"), for: .normal)
        scanQRButton.addTarget(self, action: #selector(didTapScanQR), for: .touchUpInside)
        
        // Stack View
        let stackView = UIStackView(arrangedSubviews: [
            titleLabel,
            statusLabel,
            ipLabel,
            portLabel,
            startButton,
            stopButton,
            scanQRButton
        ])
        stackView.axis = .vertical
        stackView.spacing = 20
        stackView.translatesAutoresizingMaskIntoConstraints = false
        
        view.addSubview(stackView)
        
        NSLayoutConstraint.activate([
            stackView.centerXAnchor.constraint(equalTo: view.centerXAnchor),
            stackView.centerYAnchor.constraint(equalTo: view.centerYAnchor),
            stackView.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: 40),
            stackView.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -40),
            startButton.heightAnchor.constraint(equalToConstant: 56),
            stopButton.heightAnchor.constraint(equalToConstant: 56),
            scanQRButton.heightAnchor.constraint(equalToConstant: 44)
        ])
    }
    
    @objc private func didTapScanQR() {
        let scannerVC = QRScannerViewController()
        scannerVC.modalPresentationStyle = .fullScreen
        present(scannerVC, animated: true)
    }
    
    private func startServer() {
        mirrorServer = MirrorServer()
        mirrorServer?.start(port: serverPort)
        print("[APP] Mirror server started on port \(serverPort)")
        
        setupBackgroundAudio()
    }
    
    // MARK: - Background Audio
    
    private var audioEngine: AVAudioEngine?
    private var audioPlayerNode: AVAudioPlayerNode?
    
    private func setupBackgroundAudio() {
        do {
            try AVAudioSession.sharedInstance().setCategory(.playback, mode: .default, options: [.mixWithOthers])
            try AVAudioSession.sharedInstance().setActive(true)
            
            audioEngine = AVAudioEngine()
            audioPlayerNode = AVAudioPlayerNode()
            
            guard let engine = audioEngine, let player = audioPlayerNode else { return }
            
            engine.attach(player)
            
            let format = AVAudioFormat(standardFormatWithSampleRate: 44100.0, channels: 1)!
            guard let buffer = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: 44100) else { return }
            buffer.frameLength = 44100
            
            engine.connect(player, to: engine.mainMixerNode, format: format)
            try engine.start()
            
            player.scheduleBuffer(buffer, at: nil, options: .loops, completionHandler: nil)
            player.play()
            
            print("[APP] Background silent audio playing. App should remain active.")
            
        } catch {
            print("[APP] Failed to setup background audio: \(error)")
        }
    }
    
    @objc private func startBroadcast() {
        let broadcastPicker = RPSystemBroadcastPickerView(frame: CGRect(x: 0, y: 0, width: 60, height: 60))
        broadcastPicker.preferredExtension = Bundle.main.bundleIdentifier?.appending(".BroadcastExtension")
        broadcastPicker.showsMicrophoneButton = false
        
        for subview in broadcastPicker.subviews {
            if let button = subview as? UIButton {
                button.sendActions(for: .touchUpInside)
            }
        }
        
        statusLabel.text = "Broadcasting..."
        statusLabel.textColor = .systemGreen
        startButton.isHidden = true
        stopButton.isHidden = false
    }
    
    @objc private func stopBroadcast() {
        statusLabel.text = "Ready to broadcast"
        statusLabel.textColor = .secondaryLabel
        startButton.isHidden = false
        stopButton.isHidden = true
    }
}
