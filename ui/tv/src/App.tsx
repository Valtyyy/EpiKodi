import { useEffect, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppError, PingReply, SettingsSnapshot } from "@om/shared";
import { formatError, formatPing } from "./format";

const DECORATIONS_KEY = "interface.window_decorations";

function readDecorations(snapshot: SettingsSnapshot): boolean {
  const entry = snapshot.entries.find((e) => e.descriptor.key === DECORATIONS_KEY);
  return entry?.value.type === "bool" ? entry.value.value : true;
}

function App() {
  const [name, setName] = useState("");
  const [status, setStatus] = useState("");

  const [decorations, setDecorations] = useState(true);

  useEffect(() => {
    invoke<SettingsSnapshot>("get_settings")
      .then((s) => setDecorations(readDecorations(s)))
      .catch((err) => setStatus(formatError(err as AppError)));
  }, []);

  async function onToggleDecorations(enabled: boolean) {
    try {
      const next = await invoke<SettingsSnapshot>("update_settings", {
        patch: {
          changes: [{ key: DECORATIONS_KEY, value: { type: "bool", value: enabled } }],
        },
      });
      setDecorations(readDecorations(next));
    } catch (err) {
      setStatus(formatError(err as AppError));
    }
  }

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
      <label>
        <input
          type="checkbox"
          checked={decorations}
          onChange={(ev) => void onToggleDecorations(ev.currentTarget.checked)}
        />
        Show window title bar
      </label>
      <p role="status" aria-live="polite">
        {status}
      </p>
    </main>
  );
}

export default App;
