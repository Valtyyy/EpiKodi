import { useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppError, PingReply } from "@om/shared";
import { formatError, formatPing } from "./format";

function App() {
  const [name, setName] = useState("");
  const [status, setStatus] = useState("");

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    try {
      const reply = await invoke<PingReply>("ping", { name });
      setStatus(formatPing(reply));
    } catch (err) {
      setStatus(formatError(err as AppError));
    }
  }

  return (
    <main>
      <h1>Open Mosaik</h1>
      <form onSubmit={onSubmit}>
        <label htmlFor="ping-name">Name</label>
        <input id="ping-name" value={name} onChange={(ev) => setName(ev.currentTarget.value)} />
        <button type="submit">Ping</button>
      </form>
      <p role="status" aria-live="polite">
        {status}
      </p>
    </main>
  );
}

export default App;
