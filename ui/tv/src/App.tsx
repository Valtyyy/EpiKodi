import { useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";

// Local types for now; a later step generates them from Rust with ts-rs.
interface PingReply {
  message: string;
  timestamp_ms: number;
}
interface AppError {
  code: string;
  message: string;
  field?: string;
  suggestion?: string;
}

function App() {
  const [name, setName] = useState("");
  const [status, setStatus] = useState("");

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    try {
      const reply = await invoke<PingReply>("ping", { name });
      setStatus(`${reply.message} (${reply.timestamp_ms})`);
    } catch (err) {
      const e = err as AppError;
      setStatus(e.suggestion ? `${e.message}. ${e.suggestion}` : e.message);
    }
  }

  return (
    <main>
      <h1>Open Mosaik</h1>
      <form onSubmit={onSubmit}>
        <label htmlFor="ping-name">Name</label>
        <input
          id="ping-name"
          value={name}
          onChange={(ev) => setName(ev.currentTarget.value)}
        />
        <button type="submit">Ping</button>
      </form>
      <p role="status" aria-live="polite">
        {status}
      </p>
    </main>
  );
}

export default App;
