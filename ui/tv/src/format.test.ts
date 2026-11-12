import { describe, expect, it } from "vitest";
import { formatError, formatPing } from "./format";

describe("format", () => {
  it("formats a ping reply", () => {
    expect(formatPing({ message: "pong", timestamp_ms: 42 })).toBe("pong (42)");
  });

  it("appends the suggestion to an error when present", () => {
    expect(formatError({ code: "x", message: "Boom" })).toBe("Boom");
    expect(formatError({ code: "x", message: "Boom", suggestion: "Retry" })).toBe("Boom. Retry");
  });
});
