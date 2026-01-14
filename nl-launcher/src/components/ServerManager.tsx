import { invoke } from "@tauri-apps/api/core";
import { open } from '@tauri-apps/plugin-dialog';
import { Power, Upload } from "lucide-react";
import { useState } from "react";

interface Props {
  selectedSerial: string | null;
}

export function ServerManager({ selectedSerial }: Props) {
  const [installMsg, setInstallMsg] = useState("");
  const [loading, setLoading] = useState(false);

  async function handleInstall() {
    if (!selectedSerial) return;
    try {
      const file = await open({
        multiple: false,
        filters: [{ name: 'Android Package', extensions: ['apk'] }]
      });

      if (!file) return;
      setLoading(true);
      setInstallMsg("Installing...");

      // Note: 'open' returns path or null. Depending on API version it might be object.
      // In v2 plugin-dialog, it returns string | string[] | null

      const path = file as string; // Assume single string
      await invoke("install_apk", { serial: selectedSerial, path });
      setInstallMsg("Installed Successfully!");

      setTimeout(() => setInstallMsg(""), 3000);
    } catch (e) {
      setInstallMsg(`Error: ${e}`);
    } finally {
      setLoading(false);
    }
  }

  async function handleStop() {
    if (!selectedSerial) return;
    await invoke("stop_server", { serial: selectedSerial });
    setInstallMsg("Server Stopped.");
    setTimeout(() => setInstallMsg(""), 2000);
  }

  return (
    <div className="tools-grid">
      <div className="tool-card">
        <div className="tool-title"><Upload size={16} /> Install APK</div>
        <div className="tool-desc">Push and install an APK file to the device.</div>
        <button className="btn-primary tool-btn" onClick={handleInstall} disabled={!selectedSerial || loading}>
          Select APK
        </button>
      </div>

      <div className="tool-card">
        <div className="tool-title" style={{ color: 'var(--error)' }}><Power size={16} /> Stop Server</div>
        <div className="tool-desc">Kill the mirror server process on device.</div>
        <button
          className="btn-primary tool-btn"
          style={{ backgroundColor: 'rgba(239, 68, 68, 0.2)', color: 'var(--error)' }}
          onClick={handleStop}
          disabled={!selectedSerial}>
          Kill Process
        </button>
      </div>

      {installMsg && (
        <div style={{ gridColumn: 'span 2', fontSize: 13, color: 'var(--primary)' }}>
          {installMsg}
        </div>
      )}
    </div>
  );
}
