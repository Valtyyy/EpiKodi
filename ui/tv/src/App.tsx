import { useState } from "react";
import { ApiProvider, type SettingsSnapshot, type TvApiClient } from "@om/shared";
import { createTauriApiClient } from "./api/tauri";
import { useSettings } from "./api/settings";
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
  const settings = useSettings();
  // Set when the wizard is left without its completion being saved (the user chose to go on).
  const [wizardDone, setWizardDone] = useState(false);

  if (settings.error) {
    return (
      <main className="p-8">
        <p role="alert" className="text-tv-md text-danger">
          {formatError(settings.error)}
        </p>
      </main>
    );
  }
  if (!settings.data) {
    return (
      <main className="p-8">
        <p role="status" className="text-tv-md text-content-muted">
          Chargement…
        </p>
      </main>
    );
  }
  if (needsWizard(settings.data) && !wizardDone) {
    return <FirstRunWizard onDone={() => setWizardDone(true)} />;
  }
  return <SettingsScreen />;
}

function App({ client = tauriClient }: { client?: TvApiClient }) {
  return (
    <ApiProvider client={client}>
      <TvInput>
        <Root />
      </TvInput>
    </ApiProvider>
  );
}

export default App;
