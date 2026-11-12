import { describe, expect, it } from "vitest";
import type { AppError, PingReply } from "./index";

describe("@om/shared", () => {
  it("exposes the generated types (compile-time check)", () => {
    const err: AppError = { code: "invalid", message: "bad", field: "name" };
    const reply: PingReply = { message: "pong", timestamp_ms: 1 };
    expect(err.code).toBe("invalid");
    expect(reply.timestamp_ms).toBe(1);
  });
});
