import { invoke } from "@tauri-apps/api/core";
import { Plus, RefreshCw, Smartphone, WifiOff, X } from "lucide-react";
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { QRCodeConnect } from "./QRCodeConnect";

interface Device {
  serial: string;
  state: string;
  model: string;
}

interface Props {
  onSelect: (device: Device | null) => void;
  selectedSerial: string | null;
}

export function DeviceList({ onSelect, selectedSerial }: Props) {
  const [devices, setDevices] = useState<Device[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showAddDevice, setShowAddDevice] = useState(false);

  async function fetchDevices(silent = false) {
    if (!silent) setLoading(true);
    setError(null);
    try {
      const devs = await invoke<Device[]>("get_devices");
      setDevices(devs);

      // Sync selection
      if (devs.length > 0) {
        const stillExists = devs.find(d => d.serial === selectedSerial);
        if (!selectedSerial || !stillExists) {
          onSelect(devs[0]);
        }
      } else {
        onSelect(null);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      if (!silent) setLoading(false);
    }
  }

  useEffect(() => {
    fetchDevices();

    const unlistenDeviceChanged = listen("device-changed", () => {
      console.log("Device changed event received");
      fetchDevices(true);
    });

    const unlistenPairSuccess = listen<string>("pair-success", (event) => {
      console.log("Pair success event received for IP:", event.payload);
      fetchDevices(false);
      setShowAddDevice(false); // Auto-close QR section on success
    });

    return () => {
      unlistenDeviceChanged.then(f => f());
      unlistenPairSuccess.then(f => f());
    };
  }, []);

  return (
    <div className="panel" style={{ flexGrow: 1, display: 'flex', flexDirection: 'column', boxSizing: 'border-box' }}>
      <div className="panel-header">
        <span className="panel-title">Devices</span>
        <div style={{ display: 'flex', gap: 4 }}>
          <button
            onClick={() => setShowAddDevice(!showAddDevice)}
            title={showAddDevice ? "Close" : "Add Device"}
            style={{
              background: 'none',
              border: 'none',
              color: showAddDevice ? 'var(--error)' : 'var(--text-muted)',
              cursor: 'pointer',
              padding: 4
            }}
          >
            {showAddDevice ? <X size={16} /> : <Plus size={16} />}
          </button>
          <button
            className="refresh-btn"
            onClick={() => fetchDevices(false)}
            disabled={loading}
            style={{
              background: 'none',
              border: 'none',
              color: 'var(--text-muted)',
              cursor: 'pointer',
              padding: 4
            }}
          >
            <RefreshCw size={16} className={loading ? "loading-spinner" : ""} />
          </button>
        </div>
      </div>

      {/* Add Device Section (Collapsible) */}
      {showAddDevice && (
        <div style={{ marginBottom: 16, paddingBottom: 16, borderBottom: '1px solid var(--border-subtle)' }}>
          <QRCodeConnect />
        </div>
      )}

      <div className="device-list">
        {error && (
          <div style={{ color: 'var(--error)', fontSize: 13, padding: 10, background: 'rgba(239, 68, 68, 0.1)', borderRadius: 6 }}>
            {error}
          </div>
        )}

        {devices.length === 0 && !loading && !showAddDevice && (
          <div style={{ padding: 20, textAlign: 'center', color: 'var(--text-muted)', fontSize: 13 }}>
            <p>No devices detected.</p>
            <p style={{ fontSize: 11, opacity: 0.7 }}>Connect via USB or click "Add" for WiFi.</p>
          </div>
        )}

        {devices.map((d) => (
          <div
            key={d.serial}
            className={`device-item ${selectedSerial === d.serial ? 'selected' : ''} fade-in`}
            onClick={() => onSelect(d)}
          >
            <div className="device-icon-wrapper">
              <Smartphone size={20} />
            </div>

            <div className="device-meta">
              <span className="device-model">{d.model}</span>
              <span className="device-serial">{d.serial}</span>
            </div>

            <div className="device-status">
              {d.state === 'device' ? (
                <div className="status-dot online" title="Online" />
              ) : (
                <WifiOff size={14} color="var(--text-muted)" />
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
