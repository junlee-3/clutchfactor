import { describe, expect, it } from "vitest";
import { mapName } from "./mapName";

describe("mapName", () => {
  it("strips the de_ prefix and capitalizes the first letter", () => {
    expect(mapName("de_mirage")).toBe("Mirage");
  });

  // Rust parity (cf-narrator's map_name, templates.rs): capitalize() only
  // uppercases the string's first character — it does not insert a space
  // before the trailing digit ("Dust2", not "Dust 2").
  it("keeps a compound map slug as one capitalized word", () => {
    expect(mapName("de_dust2")).toBe("Dust2");
  });

  it("strips cs_ and ar_ prefixes too", () => {
    expect(mapName("cs_office")).toBe("Office");
    expect(mapName("ar_baggage")).toBe("Baggage");
  });

  it("passes an unrecognized map through un-prefixed", () => {
    expect(mapName("de_foo")).toBe("Foo");
  });

  it("replaces underscores elsewhere in the slug with spaces", () => {
    expect(mapName("de_st_marc")).toBe("St marc");
  });

  it("passes a map with no known prefix through capitalized as-is", () => {
    expect(mapName("workshop_map")).toBe("Workshop map");
  });

  it("trims surrounding whitespace", () => {
    expect(mapName("  de_nuke  ")).toBe("Nuke");
  });

  it("returns an empty string for empty input", () => {
    expect(mapName("")).toBe("");
  });
});
