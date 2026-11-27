import { ApiProvider, type ApiClient } from "@om/shared";
import { createHttpApiClient } from "./api/http";

const httpClient = createHttpApiClient();

export function App({ client = httpClient }: { client?: ApiClient }) {
  return (
    <ApiProvider client={client}>
      <main>
        <h1>Open Mosaik</h1>
        <p>Télécommande en cours de construction.</p>
      </main>
    </ApiProvider>
  );
}
