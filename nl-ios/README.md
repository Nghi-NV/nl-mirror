# nl-ios

iOS Screen Mirroring App using ReplayKit Broadcast Extension.

## Features

- 📺 Real-time screen capture via ReplayKit
- 🎬 H.264 hardware encoding via VideoToolbox
- 🌐 TCP streaming to nl-host
- 📱 Supports iOS 14.0+

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     iOS Device                          │
│  ┌─────────────────┐     ┌───────────────────────────┐ │
│  │ Broadcast       │────▶│ SharedBuffer              │ │
│  │ Extension       │     │ (App Group)               │ │
│  │ - ReplayKit     │     └───────────┬───────────────┘ │
│  │ - H264Encoder   │                 │                 │
│  └─────────────────┘                 ▼                 │
│                          ┌───────────────────────────┐ │
│                          │ Main App                  │ │
│                          │ - StreamForwarder         │ │
│                          │ - MirrorServer (TCP:9999) │ │
│                          └───────────┬───────────────┘ │
└──────────────────────────────────────┼─────────────────┘
                                       │
                                       ▼ TCP Stream
                              ┌─────────────────┐
                              │    nl-host      │
                              │    (macOS)      │
                              └─────────────────┘
```

## Usage

1. Build and install on iOS device via Xcode
2. Open NL-iOS Mirror app
3. Note the IP address displayed
4. Tap "Start Broadcast"
5. Select "NL Broadcast" from the broadcast picker
6. On your Mac, connect with nl-host:
   ```bash
   ./nl-host mirror-ios --host <DEVICE_IP> --port 9999
   ```

## Requirements

- Xcode 15.0+
- iOS 14.0+
- Apple Developer account (for device installation)

## Project Structure

```
nl-ios/
├── nl-ios.xcodeproj/
├── nl-ios/                      # Main App
│   ├── AppDelegate.swift
│   ├── ViewController.swift
│   ├── MirrorServer.swift
│   ├── StreamForwarder.swift
│   └── Info.plist
├── BroadcastExtension/          # ReplayKit Extension
│   ├── SampleHandler.swift
│   ├── H264Encoder.swift
│   └── Info.plist
└── Shared/                      # Shared Code
    └── SharedBuffer.swift
```

## App Group Configuration

Both the main app and broadcast extension must have the same App Group:
- `group.dev.nl.ios.mirror`

This is configured in the `.entitlements` files.

## Signing

Before building, set your Development Team in Xcode:
1. Select the project in the navigator
2. Select each target (nl-ios, BroadcastExtension)
3. Under Signing & Capabilities, set your Team

## License

MIT
