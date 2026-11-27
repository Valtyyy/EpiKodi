import { describe, expect, it, vi } from "vitest";
import { createHttpApiClient } from "./http";

const json = (status: number, body: unknown) =>
  new Response(JSON.stringify(body), { status, headers: { "Content-Type": "application/json" } });

describe("HttpApiClient", () => {
  it("posts the action to the control API with the session cookie", async () => {
    const fetch = vi.fn(async () => new Response(null, { status: 204 }));
    await createHttpApiClient({ fetch }).sendInput("ok");
    expect(fetch).toHaveBeenCalledWith("/api/v1/input", {
      method: "POST",
      credentials: "same-origin",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ action: "ok" }),
    });
  });

  it("passes the server's AppError through, field and suggestion included", async () => {
    const error = {
      code: "unpaired",
      message: "Appareil non appairé",
      suggestion: "Scannez le QR code",
    };
    const fetch = vi.fn(async () => json(401, error));
    await expect(createHttpApiClient({ fetch }).sendInput("up")).rejects.toEqual(error);
  });

  it("falls back to the status when the body is not an AppError", async () => {
    const fetch = vi.fn(async () => new Response("<html>Bad gateway</html>", { status: 502 }));
    await expect(createHttpApiClient({ fetch }).sendInput("up")).rejects.toMatchObject({
      code: "http_502",
    });
  });

  it("reports an unreachable TV as a network error", async () => {
    const fetch = vi.fn(async () => {
      throw new TypeError("Failed to fetch");
    });
    await expect(createHttpApiClient({ fetch }).sendInput("up")).rejects.toMatchObject({
      code: "network",
    });
  });
});
