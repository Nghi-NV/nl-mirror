import { MonitorPlay } from "lucide-react";
import { useState } from "react";
import "./App.css";
import { DeviceList } from "./components/DeviceList";
import { KeyboardShortcuts } from "./components/KeyboardShortcuts";
import { MirrorControls } from "./components/MirrorControls";

interface Device {
  serial: string;
  state: string;
  model: string;
}

function App() {
  const [selectedDevice, setSelectedDevice] = useState<Device | null>(null);

  return (
    <div className="app-container">
      <header>
        <div className="logo-box">
          <MonitorPlay color="white" size={24} />
        </div>
        <div>
          <h1>NL-Mirror</h1>
          <p className="subtitle">High-fidelity Android Streaming & Control</p>
        </div>
      </header>

      <main className="main-content">
        <DeviceList
          onSelect={setSelectedDevice}
          selectedSerial={selectedDevice?.serial || null}
        />

        <MirrorControls selectedSerial={selectedDevice?.serial || null} />
      </main>

      <KeyboardShortcuts />
    </div>
  );
}

export default App;

