import { describe, expect, it } from "vitest";
import { basename } from "./basename";

describe("basename", () => {
  it("handles Windows separators", () => {
    expect(basename("C:\\demos\\a.dem")).toBe("a.dem");
  });

  it("handles POSIX separators", () => {
    expect(basename("/x/y/b.dem")).toBe("b.dem");
  });

  it("passes through bare names", () => {
    expect(basename("plain.dem")).toBe("plain.dem");
  });

  it("ignores trailing separators", () => {
    expect(basename("/x/y/")).toBe("y");
  });
});
