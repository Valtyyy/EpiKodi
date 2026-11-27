import type { ApiClient, AppError } from "@om/shared";

export interface HttpApiClientOptions {
  /** Control API prefix (ARCHITECTURE.md section 8). Same origin: the page is served by it. */
  baseUrl?: string;
  /** Injected for tests. */
  fetch?: typeof globalThis.fetch;
}

function isAppError(body: unknown): body is AppError {
  return (
    typeof body === "object" &&
    body !== null &&
    typeof (body as AppError).code === "string" &&
    typeof (body as AppError).message === "string"
  );
}

async function toAppError(res: Response): Promise<AppError> {
  try {
    const body: unknown = await res.json();
    if (isAppError(body)) return body;
  } catch {
    // Not JSON (proxy page, empty body): fall back to the status.
  }
  return { code: `http_${res.status}`, message: `Erreur du serveur (${res.status}).` };
}

/**
 * `ApiClient` over the control API. The session cookie set at pairing is sent by the
 * browser (`credentials: "same-origin"`); no header can be added to the WebSocket anyway.
 * Plain HTTP on the LAN: only `fetch`, no Service Worker or `crypto.subtle`.
 */
export function createHttpApiClient(options: HttpApiClientOptions = {}): ApiClient {
  const baseUrl = options.baseUrl ?? "/api/v1";
  // Wrapped: calling an unbound `window.fetch` throws "Illegal invocation".
  const fetchFn = options.fetch ?? ((input, init) => globalThis.fetch(input, init));

  async function send(method: string, path: string, body?: unknown): Promise<Response> {
    let res: Response;
    try {
      res = await fetchFn(`${baseUrl}${path}`, {
        method,
        credentials: "same-origin",
        headers: body === undefined ? undefined : { "Content-Type": "application/json" },
        body: body === undefined ? undefined : JSON.stringify(body),
      });
    } catch {
      const error: AppError = {
        code: "network",
        message: "La télé est injoignable.",
        suggestion: "Vérifiez que le téléphone est sur le même réseau que la télé.",
      };
      throw error;
    }
    if (!res.ok) throw await toAppError(res);
    return res;
  }

  return {
    // Route implemented by #80; the body mirrors `UiEvent::Input`.
    sendInput: async (action) => {
      await send("POST", "/input", { action });
    },
  };
}
