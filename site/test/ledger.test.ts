import { describe, expect, it } from "vitest";
import { ledgerSchedule, parseClock } from "../src/ledger";

describe("parseClock", () => {
  it("m:ss → seconds", () => {
    expect(parseClock("0:05")).toBe(5);
    expect(parseClock("1:12")).toBe(72);
  });
});

describe("ledgerSchedule", () => {
  it("a single row appears at 400 ms", () => {
    expect(ledgerSchedule([{ t: "0:05" }])).toEqual([400]);
  });

  it("is linear in the timestamp between 400 ms and 3600 ms", () => {
    expect(ledgerSchedule([{ t: "0:00" }, { t: "0:10" }, { t: "0:20" }])).toEqual([400, 2000, 3600]);
  });

  it("staggers equal timestamps by 250 ms", () => {
    expect(ledgerSchedule([{ t: "0:05" }, { t: "1:12" }, { t: "1:12" }])).toEqual([400, 3600, 3850]);
    expect(ledgerSchedule([{ t: "0:05" }, { t: "0:05" }, { t: "0:05" }])).toEqual([400, 650, 900]);
  });

  it("schedules the spec's round-2 ledger", () => {
    const rows = [{ t: "0:05" }, { t: "0:31" }, { t: "0:55" }, { t: "1:01" }, { t: "1:12" }, { t: "1:12" }];
    expect(ledgerSchedule(rows)).toEqual([400, 1642, 2788, 3075, 3600, 3850]);
  });

  it("empty input → empty schedule", () => {
    expect(ledgerSchedule([])).toEqual([]);
  });
});
