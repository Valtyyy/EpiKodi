import { ApiProvider } from "@om/shared";
import { createTauriApiClient } from "./api/tauri";
import { SettingsScreen } from "./settings/SettingsScreen";

const client = createTauriApiClient();

function App() {
  return (
    <ApiProvider client={client}>
      <SettingsScreen />
    </ApiProvider>
  );
}

export default App;
