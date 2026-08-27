import { describe, expect, it } from "vitest";
import { trackedInitials, trackedLabel } from "./trackedPlayer";

const base = { steamid: "76561199228328773", name: null, avatar: null };

describe("trackedLabel", () => {
  it("prefers the name", () => {
    expect(trackedLabel({ ...base, name: "misosoupy3" })).toBe("misosoupy3");
  });

  it("shortens the steamid when there is no name", () => {
    expect(trackedLabel(base)).toBe("7656…8773");
  });

  it("treats a blank name as no name", () => {
    expect(trackedLabel({ ...base, name: "   " })).toBe("7656…8773");
  });

  it("falls back to Unknown player with nothing to show", () => {
    expect(trackedLabel({ ...base, steamid: "" })).toBe("Unknown player");
  });
});

describe("trackedInitials", () => {
  it("takes the first two letters of a single word", () => {
    expect(trackedInitials("misosoupy3")).toBe("MI");
  });

  it("takes one letter per word when there are several", () => {
    expect(trackedInitials("Jun Lee")).toBe("JL");
    expect(trackedInitials("s1mple_navi")).toBe("SN");
  });

  it("skips leading punctuation clan tags leave behind", () => {
    expect(trackedInitials("-=zoot=-")).toBe("ZO");
  });

  it("survives a name with no letters at all", () => {
    expect(trackedInitials("♥")).toBe("♥");
    expect(trackedInitials("")).toBe("?");
  });
});
