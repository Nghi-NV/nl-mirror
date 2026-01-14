import { invoke } from "@tauri-apps/api/core";
import { Link, Wifi } from "lucide-react";
import { useState } from "react";

interface Props {
  selectedSerial: string | null;
}

export function WiFiConnect({ selectedSerial }: Props) {
  const [ip, setIp] = useState("");
  const [loading, setLoading] = useState(false);
  const [msg, setMsg] = useState<{ txt: string, err: boolean } | null>(null);

  async function handleEnableWifi() {
    if (!selectedSerial) return;
    setLoading(true);
    try {
      const res = await invoke<string>("enable_wifi", { serial: selectedSerial });
      setMsg({ txt: res, err: false });
    } catch (e) {
      setMsg({ txt: String(e), err: true });
    } finally {
      setLoading(false);
    }
  }

  async function handleConnect() {
    if (!ip) return;
    setLoading(true);
    try {
      const res = await invoke<string>("connect_wifi", { ip });
      setMsg({ txt: res, err: false });
    } catch (e) {
      setMsg({ txt: String(e), err: true });
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="tools-grid">
      <div className="tool-card">
        <div className="tool-title"><Wifi size={16} /> Enable WiFi Mode</div>
        <div className="tool-desc">Switch device to TCP/IP mode (5555). Requires USB.</div>
        <button className="btn-primary tool-btn" onClick={handleEnableWifi} disabled={!selectedSerial || loading}>
          Enable on Device
        </button>
      </div>

      <div className="tool-card">
        <div className="tool-title"><Link size={16} /> Connect via IP</div>
        <div className="tool-desc">Connect to a device running in TCP/IP mode.</div>
        <div className="input-group">
          <input
            className="tool-input"
            placeholder="192.168.1.x"
            value={ip}
            onChange={(e) => setIp(e.target.value)}
          />
          <button className="btn-primary tool-btn" onClick={handleConnect} disabled={!ip || loading}>
            Connect
          </button>
        </div>
      </div>

      {msg && (
        <div style={{ gridColumn: 'span 2', fontSize: 13, color: msg.err ? 'var(--error)' : 'var(--success)' }}>
          {msg.txt}
        </div>
      )}
    </div>
  );
}
