import { describe, expect, it } from "vitest";
import { errorMessage } from "./errors";

describe("errorMessage", () => {
  it("uses an Error's message, a string as-is, and a calm fallback otherwise", () => {
    expect(errorMessage(new Error("store lock poisoned"))).toBe("store lock poisoned");
    expect(errorMessage("no such match")).toBe("no such match");
    expect(errorMessage({ weird: true })).toBe("something went wrong on the Rust side — the log has the details");
    expect(errorMessage(undefined)).toBe("something went wrong on the Rust side — the log has the details");
  });
  it("trims and caps very long messages so the empty state stays readable", () => {
    expect(errorMessage("x".repeat(500))).toHaveLength(200);
  });
});
