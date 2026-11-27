import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTvApi, type SecretStatus, type SettingsPatch, type SettingsSnapshot } from "@om/shared";

/** One cached snapshot shared by every screen: a write updates it, nobody refetches by hand. */
export const SETTINGS_KEY = ["settings"] as const;

export function useSettings() {
  const api = useTvApi();
  return useQuery({
    queryKey: SETTINGS_KEY,
    queryFn: () => api.getSettings(),
    // Settings only change through this UI (every write below updates the cache) and the
    // control API has no settings route (section 8): never refetch on a screen change.
    staleTime: Infinity,
  });
}

/** `updateSettings` returns the whole snapshot: it replaces the cache as is. */
export function useUpdateSettings() {
  const api = useTvApi();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (patch: SettingsPatch) => api.updateSettings(patch),
    onSuccess: (snapshot) => queryClient.setQueryData(SETTINGS_KEY, snapshot),
  });
}

/** Secret writes only return that secret's status: merge it into the cached snapshot. */
export function useSecretStatusUpdate(): (status: SecretStatus) => void {
  const queryClient = useQueryClient();
  return (status) =>
    queryClient.setQueryData<SettingsSnapshot>(
      SETTINGS_KEY,
      (s) => s && { ...s, secrets: s.secrets.map((x) => (x.key === status.key ? status : x)) },
    );
}
