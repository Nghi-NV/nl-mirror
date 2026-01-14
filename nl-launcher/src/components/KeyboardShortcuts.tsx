import { Keyboard, X } from "lucide-react";
import { useState } from "react";

const shortcuts = [
  {
    category: "General", items: [
      { key: "Cmd/Ctrl + S", desc: "Screenshot (saves to Desktop)" },
      { key: "Cmd/Ctrl + C", desc: "Copy from Android clipboard" },
      { key: "Cmd/Ctrl + V", desc: "Paste to Android" },
    ]
  },
  {
    category: "Navigation", items: [
      { key: "ESC", desc: "Back" },
      { key: "F1", desc: "Home" },
      { key: "F2", desc: "Recent Apps" },
      { key: "F6", desc: "Menu" },
    ]
  },
  {
    category: "Media & Volume", items: [
      { key: "F3", desc: "Volume Down" },
      { key: "F4", desc: "Volume Up" },
      { key: "F5", desc: "Power (toggle screen)" },
    ]
  },
  {
    category: "Screen Control", items: [
      { key: "F7", desc: "Screen OFF (device only)" },
      { key: "F8", desc: "Screen ON (device only)" },
    ]
  },
  {
    category: "Gestures", items: [
      { key: "F9", desc: "Swipe Up (next video)" },
      { key: "F10", desc: "Swipe Down (prev video)" },
    ]
  },
];

export function KeyboardShortcuts() {
  const [isOpen, setIsOpen] = useState(false);

  return (
    <>
      <button
        className="help-btn"
        onClick={() => setIsOpen(true)}
        title="Keyboard Shortcuts"
      >
        <Keyboard size={18} />
      </button>

      {isOpen && (
        <div className="modal-overlay" onClick={() => setIsOpen(false)}>
          <div className="modal-content shortcuts-modal" onClick={e => e.stopPropagation()}>
            <div className="modal-header">
              <h2><Keyboard size={20} /> Keyboard Shortcuts</h2>
              <button className="close-btn" onClick={() => setIsOpen(false)}>
                <X size={18} />
              </button>
            </div>
            <div className="shortcuts-grid">
              {shortcuts.map(cat => (
                <div key={cat.category} className="shortcut-category">
                  <h3>{cat.category}</h3>
                  <ul>
                    {cat.items.map(item => (
                      <li key={item.key}>
                        <kbd>{item.key}</kbd>
                        <span>{item.desc}</span>
                      </li>
                    ))}
                  </ul>
                </div>
              ))}
            </div>
            <div className="modal-footer">
              <span className="hint">These shortcuts work in the Mirror window</span>
            </div>
          </div>
        </div>
      )}

      <style>{`
        .help-btn {
          position: fixed;
          bottom: 20px;
          right: 20px;
          width: 44px;
          height: 44px;
          border-radius: 50%;
          background: var(--primary);
          border: none;
          color: white;
          cursor: pointer;
          display: flex;
          align-items: center;
          justify-content: center;
          box-shadow: 0 4px 12px rgba(59, 130, 246, 0.4);
          transition: all 0.2s;
          z-index: 100;
        }
        .help-btn:hover {
          transform: scale(1.1);
          box-shadow: 0 6px 20px rgba(59, 130, 246, 0.5);
        }

        .modal-overlay {
          position: fixed;
          inset: 0;
          background: rgba(0,0,0,0.7);
          display: flex;
          align-items: center;
          justify-content: center;
          z-index: 1000;
          animation: fadeIn 0.15s ease-out;
        }
        
        @keyframes fadeIn {
          from { opacity: 0; }
          to { opacity: 1; }
        }

        .modal-content {
          background: #09090b;
          border: 1px solid var(--border-subtle);
          border-radius: var(--radius-lg);
          max-width: 600px;
          max-height: 80vh;
          overflow: auto;
          box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.5), 0 10px 10px -5px rgba(0, 0, 0, 0.4);
          animation: slideUp 0.2s ease-out;
        }

        @keyframes slideUp {
          from { transform: translateY(20px); opacity: 0; }
          to { transform: translateY(0); opacity: 1; }
        }

        .modal-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 16px 20px;
          border-bottom: 1px solid var(--border-subtle);
        }
        .modal-header h2 {
          display: flex;
          align-items: center;
          gap: 10px;
          margin: 0;
          font-size: 16px;
          font-weight: 600;
        }
        .close-btn {
          background: transparent;
          border: none;
          color: var(--text-muted);
          cursor: pointer;
          padding: 4px;
          border-radius: 4px;
        }
        .close-btn:hover {
          background: var(--bg-hover);
          color: white;
        }

        .shortcuts-grid {
          display: grid;
          grid-template-columns: repeat(2, 1fr);
          gap: 20px;
          padding: 20px;
        }

        .shortcut-category h3 {
          font-size: 12px;
          text-transform: uppercase;
          color: var(--text-muted);
          margin: 0 0 10px 0;
          letter-spacing: 0.5px;
        }
        .shortcut-category ul {
          list-style: none;
          margin: 0;
          padding: 0;
        }
        .shortcut-category li {
          display: flex;
          align-items: center;
          gap: 12px;
          padding: 6px 0;
          font-size: 13px;
        }
        .shortcut-category kbd {
          background: var(--bg-app);
          border: 1px solid var(--border-subtle);
          border-radius: 4px;
          padding: 3px 8px;
          font-family: monospace;
          font-size: 11px;
          color: var(--primary);
          white-space: nowrap;
          min-width: 80px;
          text-align: center;
        }
        .shortcut-category span {
          color: var(--text-normal);
        }

        .modal-footer {
          padding: 12px 20px;
          border-top: 1px solid var(--border-subtle);
          text-align: center;
        }
        .modal-footer .hint {
          font-size: 11px;
          color: var(--text-muted);
        }
      `}</style>
    </>
  );
}
