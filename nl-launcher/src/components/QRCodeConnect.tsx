import { useEffect, useState } from "react";
import QRCode from "react-qr-code";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Copy } from "lucide-react";

export function QRCodeConnect() {
  const [ip, setIp] = useState("");
  const [pairingUrl, setPairingUrl] = useState("");
  const [status, setStatus] = useState("Scan with your phone");
  const [statusType, setStatusType] = useState<'info' | 'success' | 'error'>('info');

  useEffect(() => {
    // Start pairing server when component mounts
    invoke("start_pairing_server").catch(console.error);
    fetchIp();

    const unlistenConnecting = listen("pair-connecting", (event) => {
      setStatus(`Connecting to ${event.payload}...`);
      setStatusType('info');
    });

    const unlistenSuccess = listen("pair-success", (event) => {
      setStatus(`✓ Connected to ${event.payload}!`);
      setStatusType('success');
    });

    const unlistenError = listen("pair-error", (event) => {
      setStatus(`✕ ${event.payload}`);
      setStatusType('error');
    });

    return () => {
      // Keep server running - don't stop on unmount
      // This prevents issues with HMR and component re-renders
      unlistenConnecting.then(f => f());
      unlistenSuccess.then(f => f());
      unlistenError.then(f => f());
    }
  }, []);

  async function fetchIp() {
    try {
      const res = await invoke<string>("get_local_ip");
      setIp(res);
      const url = `http://${res}:27015/pair`;
      setPairingUrl(url);
    } catch (e) {
      console.error(e);
      setStatus("Failed to get local IP");
      setStatusType('error');
    }
  }

  return (
    <div className="tool-card" style={{ display: "flex", flexDirection: "row", gap: 20, alignItems: "center", flexWrap: "wrap" }}>
      <div style={{ background: "white", padding: 10, borderRadius: 8 }}>
        {pairingUrl ? (
          <QRCode value={pairingUrl} size={128} />
        ) : (
          <div style={{ width: 128, height: 128, background: "#eee", display: "flex", alignItems: "center", justifyContent: "center" }}>Loading...</div>
        )}
      </div>

      <div style={{ flex: 1 }}>
        <div className="tool-title" style={{ marginBottom: 8 }}>Wireless Connection</div>
        <div className="tool-desc" style={{ marginBottom: 12 }}>
          Scan this code with the NL Mirror mobile app (or any browser on the same WiFi) to connect instantly.
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 13, color: "var(--text-secondary)" }}>
          <span style={{ fontWeight: 600, color: statusType === 'success' ? 'var(--success)' : (statusType === 'error' ? 'var(--error)' : 'var(--primary)') }}>
            {status}
          </span>
          {statusType === 'info' && status !== 'Scan with your phone' && <div className="loading-spinner" style={{ width: 12, height: 12, border: '2px solid var(--primary)', borderRadius: '50%', borderTopColor: 'transparent' }}></div>}
        </div>

        {ip && (
          <div style={{ marginTop: 12, padding: "8px 12px", background: "rgba(255,255,255,0.05)", borderRadius: 6, fontSize: 12, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span style={{ fontFamily: "monospace" }}>{pairingUrl}</span>
            <button className="icon-btn" onClick={() => navigator.clipboard.writeText(pairingUrl)} title="Copy URL">
              <Copy size={14} />
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
