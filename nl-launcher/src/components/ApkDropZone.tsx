import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Download, CheckCircle, AlertCircle, Loader2 } from "lucide-react";
import { useEffect, useState, useCallback } from "react";

interface Props {
  selectedSerial: string | null;
}

type InstallStatus = 
  | { type: 'idle' }
  | { type: 'hovering' }
  | { type: 'installing'; fileName: string }
  | { type: 'success'; fileName: string }
  | { type: 'error'; message: string };

export function ApkDropZone({ selectedSerial }: Props) {
  const [status, setStatus] = useState<InstallStatus>({ type: 'idle' });

  const handleInstall = useCallback(async (filePath: string) => {
    if (!selectedSerial) {
      setStatus({ type: 'error', message: "No device selected" });
      setTimeout(() => setStatus({ type: 'idle' }), 3000);
      return;
    }

    // Validate .apk extension
    if (!filePath.toLowerCase().endsWith('.apk')) {
      setStatus({ type: 'error', message: "Only .apk files are supported" });
      setTimeout(() => setStatus({ type: 'idle' }), 3000);
      return;
    }

    const fileName = filePath.split('/').pop() || filePath;
    setStatus({ type: 'installing', fileName });

    try {
      await invoke("install_apk", { serial: selectedSerial, path: filePath });
      setStatus({ type: 'success', fileName });
      setTimeout(() => setStatus({ type: 'idle' }), 4000);
    } catch (e) {
      setStatus({ type: 'error', message: String(e) });
      setTimeout(() => setStatus({ type: 'idle' }), 5000);
    }
  }, [selectedSerial]);

  useEffect(() => {
    let unlistenHover: UnlistenFn | null = null;
    let unlistenDrop: UnlistenFn | null = null;
    let unlistenCancel: UnlistenFn | null = null;

    const setupListeners = async () => {
      // Tauri 2 file drop events
      unlistenHover = await listen<{ paths: string[] }>('tauri://drag-over', () => {
        setStatus({ type: 'hovering' });
      });

      unlistenDrop = await listen<{ paths: string[] }>('tauri://drag-drop', (event) => {
        const paths = event.payload.paths;
        if (paths && paths.length > 0) {
          // Find first .apk file
          const apkFile = paths.find(p => p.toLowerCase().endsWith('.apk'));
          if (apkFile) {
            handleInstall(apkFile);
          } else {
            setStatus({ type: 'error', message: "No APK file found in dropped files" });
            setTimeout(() => setStatus({ type: 'idle' }), 3000);
          }
        }
      });

      unlistenCancel = await listen('tauri://drag-leave', () => {
        if (status.type === 'hovering') {
          setStatus({ type: 'idle' });
        }
      });
    };

    setupListeners();

    return () => {
      unlistenHover?.();
      unlistenDrop?.();
      unlistenCancel?.();
    };
  }, [handleInstall, status.type]);

  // Only show indicator when relevant
  if (status.type === 'idle') {
    return null;
  }

  return (
    <div className={`apk-drop-overlay ${status.type}`}>
      <div className="apk-drop-content">
        {status.type === 'hovering' && (
          <>
            <Download size={48} className="drop-icon bounce" />
            <span className="drop-text">Drop APK to Install</span>
            {!selectedSerial && (
              <span className="drop-warning">⚠️ No device selected</span>
            )}
          </>
        )}
        
        {status.type === 'installing' && (
          <>
            <Loader2 size={48} className="drop-icon spin" />
            <span className="drop-text">Installing {status.fileName}...</span>
          </>
        )}
        
        {status.type === 'success' && (
          <>
            <CheckCircle size={48} className="drop-icon success" />
            <span className="drop-text success">Installed {status.fileName}</span>
          </>
        )}
        
        {status.type === 'error' && (
          <>
            <AlertCircle size={48} className="drop-icon error" />
            <span className="drop-text error">{status.message}</span>
          </>
        )}
      </div>

      <style>{`
        .apk-drop-overlay {
          position: fixed;
          top: 0;
          left: 0;
          right: 0;
          bottom: 0;
          display: flex;
          align-items: center;
          justify-content: center;
          z-index: 9999;
          pointer-events: none;
          transition: all 0.2s ease;
        }

        .apk-drop-overlay.hovering {
          background: rgba(59, 130, 246, 0.15);
          backdrop-filter: blur(4px);
          border: 3px dashed var(--primary);
        }

        .apk-drop-overlay.installing,
        .apk-drop-overlay.success,
        .apk-drop-overlay.error {
          background: rgba(0, 0, 0, 0.7);
          backdrop-filter: blur(8px);
        }

        .apk-drop-content {
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: 16px;
          padding: 32px 48px;
          background: var(--bg-panel);
          border-radius: 16px;
          border: 1px solid var(--border-subtle);
          box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
        }

        .drop-icon {
          color: var(--primary);
        }

        .drop-icon.success {
          color: var(--success);
        }

        .drop-icon.error {
          color: var(--error);
        }

        .drop-icon.bounce {
          animation: bounce 0.6s ease infinite;
        }

        .drop-icon.spin {
          animation: spin 1s linear infinite;
        }

        @keyframes bounce {
          0%, 100% { transform: translateY(0); }
          50% { transform: translateY(-10px); }
        }

        @keyframes spin {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }

        .drop-text {
          font-size: 18px;
          font-weight: 500;
          color: var(--text-normal);
        }

        .drop-text.success {
          color: var(--success);
        }

        .drop-text.error {
          color: var(--error);
          max-width: 300px;
          text-align: center;
        }

        .drop-warning {
          font-size: 13px;
          color: var(--warning, #f59e0b);
        }
      `}</style>
    </div>
  );
}
