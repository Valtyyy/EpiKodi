import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createContext, useContext, useState, type ReactNode } from "react";
import type { AppError } from "../generated/AppError";
import { isTvApiClient, type ApiClient, type TvApiClient } from "./client";

// Every client rejects with an AppError: type query and mutation errors accordingly.
declare module "@tanstack/react-query" {
  interface Register {
    defaultError: AppError;
  }
}

const ApiContext = createContext<ApiClient | null>(null);

/** Default cache for a provider; one per provider, so tests never share cached data. */
// eslint-disable-next-line react-refresh/only-export-components -- factory colocated with its provider
export function createQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      // Backend errors are AppErrors to show (validation, locked keyring), not transient
      // faults: retrying only delays the message. Network faults are handled server-side (C6).
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
}

export interface ApiProviderProps {
  client: ApiClient;
  /** Defaults to a fresh `createQueryClient()`. */
  queryClient?: QueryClient;
  children: ReactNode;
}

/** Injects the backend client and the TanStack Query cache, like `State` on the Rust side. */
export function ApiProvider({ client, queryClient, children }: ApiProviderProps) {
  const [ownQueryClient] = useState(createQueryClient);
  return (
    <QueryClientProvider client={queryClient ?? ownQueryClient}>
      <ApiContext.Provider value={client}>{children}</ApiContext.Provider>
    </QueryClientProvider>
  );
}

// eslint-disable-next-line react-refresh/only-export-components -- hook colocated with its provider
export function useApi(): ApiClient {
  const client = useContext(ApiContext);
  if (!client) throw new Error("useApi must be used inside <ApiProvider>");
  return client;
}

/** For TV-only components; fails fast if mounted under the remote's client. */
// eslint-disable-next-line react-refresh/only-export-components -- hook colocated with its provider
export function useTvApi(): TvApiClient {
  const client = useApi();
  if (!isTvApiClient(client)) throw new Error("useTvApi needs a TvApiClient (TV only)");
  return client;
}
