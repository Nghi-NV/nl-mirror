import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { AlertCircle, Cast, CheckCircle, Usb, Wifi, Zap } from "lucide-react";
import { useEffect, useState } from "react";
import { CustomSelect } from "./ui/CustomSelect";

interface Props {
  selectedSerial: string | null;
}

export function MirrorControls({ selectedSerial }: Props) {
  const [bitrate, setBitrate] = useState(16000000); // Default 16Mbps
  const [resolution, setResolution] = useState(1080);
  const [isStarting, setIsStarting] = useState(false);
  const [status, setStatus] = useState<{ msg: string, type: 'info' | 'success' | 'error' } | null>(null);
  const [connectionMode, setConnectionMode] = useState<'usb' | 'wifi'>('usb');
  const [wifiIp, setWifiIp] = useState("");
  const [turnScreenOff, setTurnScreenOff] = useState(false);

  // Auto-scan when switching to WiFi tab
  useEffect(() => {
    if (connectionMode === 'wifi' && selectedSerial && !wifiIp) {
      handleEnableWifi();
    }
  }, [connectionMode, selectedSerial]);

  async function handleEnableWifi() {
    if (!selectedSerial) return;
    try {
      const res = await invoke<string>("enable_wifi", { serial: selectedSerial });
      if (res.includes(".")) {
        setWifiIp(res);
        setStatus({ msg: `WiFi Enabled. Device IP: ${res}`, type: 'success' });
      } else {
        setStatus({ msg: res, type: 'info' });
      }
    } catch (e) {
      // Improve error message
      const errStr = String(e);
      if (errStr.includes("not found")) {
        setStatus({ msg: "Device not found. Please check USB connection.", type: 'error' });
      } else {
        setStatus({ msg: `WiFi Enable Failed: ${errStr}`, type: 'error' });
      }
    }
  }

  const [isStreaming, setIsStreaming] = useState(false);

  useEffect(() => {
    const unlistenStop = listen('mirror-stopped', () => {
      setIsStreaming(false);
      setIsStarting(false);
      setStatus({ msg: "Mirror Stopped", type: 'success' });
      setTimeout(() => setStatus(null), 2000);
    });

    const unlistenStart = listen('mirror-started', () => {
      setIsStreaming(true);
      setIsStarting(false);
      setStatus({ msg: "Mirror Started", type: 'success' });
      setTimeout(() => setStatus(null), 2000);
    });

    return () => {
      unlistenStop.then((f: UnlistenFn) => f());
      unlistenStart.then((f: UnlistenFn) => f());
    };
  }, []);

  async function handleStop() {
    setIsStarting(true);
    try {
      await invoke("stop_mirror");
    } catch (e) {
      setStatus({ msg: `Stop Failed: ${e}`, type: 'error' });
    } finally {
      setIsStarting(false);
    }
  }

  async function handleStart() {
    if (isStreaming) {
      handleStop();
      return;
    }

    // Determine target serial (USB serial or IP)
    const target = connectionMode === 'wifi' ? (wifiIp ? `${wifiIp}:5555` : null) : selectedSerial;

    if (!target) {
      setStatus({ msg: "No device selected or IP missing.", type: 'error' });
      return;
    }

    setIsStarting(true);
    setStatus({ msg: "Initializing Session...", type: 'info' });

    try {
      if (connectionMode === 'wifi') {
        setStatus({ msg: `Connecting to ${wifiIp}...`, type: 'info' });
        await invoke("connect_wifi", { ip: wifiIp });
      }

      // 1. Unified Init (Push if needed, Forward, Start)
      // This is the "Optimized Algorithm" running in Rust
      await invoke("init_session", { serial: target });

      // 2. Client Launch
      setStatus({ msg: "Launching stream...", type: 'info' });
      await invoke("start_mirror", {
        serial: target,
        bitrate: bitrate,
        maxSize: resolution,
        turnScreenOff: turnScreenOff
      });
    } catch (e) {
      setStatus({ msg: `Failed: ${e}`, type: 'error' });
    } finally {
      setIsStarting(false);
    }
  }

  return (
    <div className="panel" style={{ display: 'flex', flexDirection: 'column', gap: 24 }}>
      <div className="panel-header" style={{ marginBottom: 0 }}>
        <span className="panel-title">Configuration</span>
        <div style={{ display: 'flex', gap: 10 }}>
          <button
            className={`tab-btn ${connectionMode === 'usb' ? 'active' : ''}`}
            onClick={() => setConnectionMode('usb')}
            title="USB Mode"
          >
            <Usb size={14} /> USB
          </button>
          <button
            className={`tab-btn ${connectionMode === 'wifi' ? 'active' : ''}`}
            onClick={() => setConnectionMode('wifi')}
            title="WiFi Mode"
          >
            <Wifi size={14} /> WiFi
          </button>
        </div>
      </div>

      <style>{`
            .tab-btn {
                background: transparent;
                border: 1px solid var(--border-subtle);
                color: var(--text-muted);
                padding: 6px 10px;
                border-radius: 6px;
                cursor: pointer;
                display: flex;
                align-items: center;
                gap: 6px;
                font-size: 12px;
            }
            .tab-btn.active {
                background: var(--primary);
                color: white;
                border-color: var(--primary);
            }
            .btn-danger {
                background: var(--error) !important;
                border-color: var(--error) !important;
            }
        `}</style>

      {connectionMode === 'wifi' && (
        <div className="form-group fade-in">
          <label>Device IP</label>
          <div className="input-group">
            <input
              value={wifiIp}
              onChange={(e) => setWifiIp(e.target.value)}
              placeholder="192.168.1.x"
              style={{ background: 'var(--bg-app)', border: '1px solid var(--border-subtle)', borderRadius: 8, padding: 10, color: 'white', outline: 'none', width: '100%' }}
            />
          </div>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <span style={{ fontSize: 11, color: 'var(--text-muted)' }}>
              {selectedSerial ? "Auto-detecting IP from USB..." : "Connect USB to auto-detect IP."}
            </span>
          </div>
        </div>
      )}

      <div className="control-grid">
        <div className="form-group">
          <label>Resolution</label>
          <CustomSelect
            value={resolution}
            onChange={(v) => setResolution(Number(v))}
            options={[
              { value: 720, label: "720p" },
              { value: 1080, label: "1080p (Rec)" },
              { value: 1440, label: "2K" },
              { value: 2160, label: "4K" }
            ]}
          />
        </div>

        <div className="form-group">
          <label>Bitrate</label>
          <CustomSelect
            value={bitrate}
            onChange={(v) => setBitrate(Number(v))}
            options={[
              { value: 2000000, label: "2 Mbps" },
              { value: 8000000, label: "8 Mbps" },
              { value: 16000000, label: "16 Mbps" },
              { value: 20000000, label: "20 Mbps" }
            ]}
          />
        </div>
      </div>

      <div className="form-group fade-in">
        <label style={{ display: 'flex', alignItems: 'center', gap: 10, cursor: 'pointer', color: 'var(--text-normal)', fontSize: 13 }}>
          <input
            type="checkbox"
            checked={turnScreenOff}
            onChange={e => setTurnScreenOff(e.target.checked)}
            style={{ accentColor: 'var(--primary)', width: 16, height: 16, cursor: 'pointer' }}
          />
          Turn screen off while mirroring
        </label>
      </div>

      <div style={{ flex: 1 }}></div>

      {status && (
        <div className={`fade-in`} style={{
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          fontSize: 13,
          color: status.type === 'error' ? 'var(--error)' : (status.type === 'success' ? 'var(--success)' : 'var(--primary)'),
          background: status.type === 'error' ? 'rgba(239, 68, 68, 0.1)' : 'rgba(59, 130, 246, 0.1)',
          padding: '10px 14px',
          borderRadius: 'var(--radius-md)'
        }}>
          {status.type === 'success' ? <CheckCircle size={16} /> : (status.type === 'error' ? <AlertCircle size={16} /> : <Zap size={16} className="loading-spinner" />)}
          {status.msg}
        </div>
      )}

      <button
        className={`btn-primary ${isStreaming ? 'btn-danger' : ''}`}
        disabled={(!selectedSerial && connectionMode === 'usb') || (connectionMode === 'wifi' && !wifiIp) || isStarting}
        onClick={handleStart}
      >
        {isStarting ? (
          <>Progressing...</>
        ) : isStreaming ? (
          <>
            <Cast size={18} />
            Stop Mirror
          </>
        ) : (
          <>
            <Cast size={18} />
            Start Stream
          </>
        )}
      </button>
      <div style={{ height: 40 }}></div>
    </div>
  );
}
