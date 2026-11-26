import { useEffect, useState } from "react";
import {
  ApiProvider,
  useApi,
  type AppError,
  type ApiClient,
  type SettingsSnapshot,
} from "@om/shared";
import { createTauriApiClient } from "./api/tauri";
import { formatError } from "./format";
import { TvInput } from "./input/TvInput";
import { SettingsScreen } from "./settings/SettingsScreen";
import { FIRST_RUN_KEY } from "./wizard/constants";
import { FirstRunWizard } from "./wizard/FirstRunWizard";

const tauriClient = createTauriApiClient();

function needsWizard(snapshot: SettingsSnapshot): boolean {
  const entry = snapshot.entries.find((e) => e.descriptor.key === FIRST_RUN_KEY);
  return entry?.value.type === "bool" && !entry.value.value;
}

function Root() {
  const api = useApi();
  const [snapshot, setSnapshot] = useState<SettingsSnapshot>();
  const [error, setError] = useState<string>();
  const [wizardDone, setWizardDone] = useState(false);

  useEffect(() => {
    api.getSettings().then(setSnapshot, (err) => setError(formatError(err as AppError)));
  }, [api]);

  if (error) {
    return (
      <main className="p-8">
        <p role="alert" className="text-tv-md text-danger">
          {error}
        </p>
      </main>
    );
  }
  if (!snapshot) {
    return (
      <main className="p-8">
        <p role="status" className="text-tv-md text-content-muted">
          Chargement…
        </p>
      </main>
    );
  }
  if (needsWizard(snapshot) && !wizardDone) {
    return <FirstRunWizard secrets={snapshot.secrets} onDone={() => setWizardDone(true)} />;
  }
  return <SettingsScreen />;
}

function App({ client = tauriClient }: { client?: ApiClient }) {
  return (
    <ApiProvider client={client}>
      <TvInput>
        <Root />
      </TvInput>
    </ApiProvider>
  );
}

export default App;
