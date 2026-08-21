import { describe, expect, it } from "vitest";
import {
  buttonClass,
  cardClass,
  chipClass,
  segTabIndex,
  type ButtonSize,
  type ButtonVariant,
  type CardEdge,
  type ChipVariant,
} from "./classes";

describe("buttonClass", () => {
  it("builds the primary/md class string", () => {
    expect(buttonClass("primary", "md")).toBe("ui-btn ui-btn-primary ui-btn-md");
  });

  it("builds the danger/sm class string", () => {
    expect(buttonClass("danger", "sm")).toBe("ui-btn ui-btn-danger ui-btn-sm");
  });

  it("builds the secondary/md class string", () => {
    expect(buttonClass("secondary", "md")).toBe("ui-btn ui-btn-secondary ui-btn-md");
  });

  it("builds the ghost/sm class string", () => {
    expect(buttonClass("ghost", "sm")).toBe("ui-btn ui-btn-ghost ui-btn-sm");
  });

  it("defaults size to md when omitted", () => {
    expect(buttonClass("primary")).toBe("ui-btn ui-btn-primary ui-btn-md");
  });

  it("falls through to primary for an unknown variant", () => {
    expect(buttonClass("bogus" as ButtonVariant, "md")).toBe(
      "ui-btn ui-btn-primary ui-btn-md",
    );
  });

  it("falls through to md for an unknown size", () => {
    expect(buttonClass("secondary", "bogus" as ButtonSize)).toBe(
      "ui-btn ui-btn-secondary ui-btn-md",
    );
  });
});

describe("cardClass", () => {
  it("returns the base class when no edge is given", () => {
    expect(cardClass()).toBe("ui-card");
  });

  it("adds the win edge modifier", () => {
    expect(cardClass("win")).toBe("ui-card ui-card-edge-win");
  });

  it("adds the loss edge modifier", () => {
    expect(cardClass("loss")).toBe("ui-card ui-card-edge-loss");
  });

  it("adds the severity edge modifier, which contains ui-card-edge-sev", () => {
    expect(cardClass("severity")).toContain("ui-card-edge-sev");
  });

  it("falls through to the base class for an unknown edge", () => {
    expect(cardClass("bogus" as CardEdge)).toBe("ui-card");
  });
});

describe("chipClass", () => {
  it("returns the base class for the default variant", () => {
    expect(chipClass("default")).toBe("ui-chip");
  });

  it("defaults to the base class when no variant is given", () => {
    expect(chipClass()).toBe("ui-chip");
  });

  it("adds the evidence modifier, which contains ui-chip-evidence", () => {
    expect(chipClass("evidence")).toContain("ui-chip-evidence");
  });

  it("adds the side-ct modifier", () => {
    expect(chipClass("side-ct")).toBe("ui-chip ui-chip-side-ct");
  });

  it("adds the side-t modifier", () => {
    expect(chipClass("side-t")).toBe("ui-chip ui-chip-side-t");
  });

  it("adds the count modifier", () => {
    expect(chipClass("count")).toBe("ui-chip ui-chip-count");
  });

  it("falls through to the base class for an unknown variant", () => {
    expect(chipClass("bogus" as ChipVariant)).toBe("ui-chip");
  });
});

describe("segTabIndex", () => {
  it("makes the active option tabbable", () => {
    expect(segTabIndex(0, 0)).toBe(0);
    expect(segTabIndex(2, 2)).toBe(0);
  });

  it("makes every other option unreachable by Tab", () => {
    expect(segTabIndex(1, 0)).toBe(-1);
    expect(segTabIndex(0, 2)).toBe(-1);
  });

  it("falls back to the first option when nothing matches (activeIndex -1)", () => {
    expect(segTabIndex(0, -1)).toBe(0);
  });

  it("keeps non-first options unreachable when nothing matches", () => {
    expect(segTabIndex(1, -1)).toBe(-1);
    expect(segTabIndex(2, -1)).toBe(-1);
  });
});
