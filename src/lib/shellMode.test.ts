import { describe, expect, it } from "vitest";
import { shellMode } from "./shellMode";

describe("shellMode", () => {
  it("collapses to the rail on an exact immersive route", () => {
    expect(shellMode("/report")).toBe("rail");
    expect(shellMode("/replay")).toBe("rail");
  });

  it("collapses to the rail on an immersive child route", () => {
    expect(shellMode("/report/abc123")).toBe("rail");
    expect(shellMode("/replay/abc123")).toBe("rail");
  });

  it("does not collapse a sibling route that merely shares the prefix string", () => {
    // "/reports".startsWith("/report") is true — this is exactly the bug a
    // segment-boundary check guards against.
    expect(shellMode("/reports")).toBe("full");
  });

  it("stays full on an unrelated route", () => {
    expect(shellMode("/")).toBe("full");
    expect(shellMode("/trends")).toBe("full");
    expect(shellMode("/corpus")).toBe("full");
    expect(shellMode("/settings")).toBe("full");
  });
});
