import { describe, expect, it } from "vitest";
import { addToast, expire, type ToastItem } from "./toast";

describe("addToast", () => {
  it("appends a toast carrying the given kind, text, and createdAt", () => {
    const list = addToast([], "status", "Imported 3 demos", 1000);
    expect(list).toHaveLength(1);
    expect(list[0]).toMatchObject({
      kind: "status",
      text: "Imported 3 demos",
      createdAt: 1000,
    });
  });

  it("preserves the error kind", () => {
    const list = addToast([], "error", "Import failed", 1000);
    expect(list[0].kind).toBe("error");
  });

  it("assigns unique ids across pushes", () => {
    let list = addToast([], "status", "a", 1000);
    list = addToast(list, "status", "b", 1001);
    list = addToast(list, "status", "c", 1002);
    const ids = list.map((t) => t.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("assigns monotonically increasing ids across pushes", () => {
    let list = addToast([], "status", "a", 1000);
    list = addToast(list, "status", "b", 1001);
    list = addToast(list, "status", "c", 1002);
    const ids = list.map((t) => t.id);
    expect(ids[0]).toBeLessThan(ids[1]);
    expect(ids[1]).toBeLessThan(ids[2]);
  });

  it("caps the visible list at 3, dropping the oldest", () => {
    let list: ToastItem[] = [];
    list = addToast(list, "status", "one", 1000);
    list = addToast(list, "status", "two", 1001);
    list = addToast(list, "status", "three", 1002);
    list = addToast(list, "status", "four", 1003);
    expect(list).toHaveLength(3);
    expect(list.map((t) => t.text)).toEqual(["two", "three", "four"]);
  });

  it("keeps ids increasing even after the cap drops older toasts", () => {
    let list: ToastItem[] = [];
    for (let i = 0; i < 5; i++) {
      list = addToast(list, "status", `t${i}`, 1000 + i);
    }
    const ids = list.map((t) => t.id);
    expect(list).toHaveLength(3);
    expect(ids).toEqual([...ids].sort((a, b) => a - b));
    expect(new Set(ids).size).toBe(3);
  });
});

describe("expire", () => {
  it("removes toasts older than the 5s TTL", () => {
    const list: ToastItem[] = [
      { id: 1, kind: "status", text: "old", createdAt: 0 },
      { id: 2, kind: "status", text: "new", createdAt: 4000 },
    ];
    expect(expire(list, 5001).map((t) => t.id)).toEqual([2]);
  });

  it("expires a toast exactly at the 5000ms TTL boundary", () => {
    const list: ToastItem[] = [{ id: 1, kind: "status", text: "x", createdAt: 0 }];
    expect(expire(list, 5000)).toHaveLength(0);
  });

  it("keeps a toast younger than the TTL", () => {
    const list: ToastItem[] = [{ id: 1, kind: "status", text: "x", createdAt: 0 }];
    expect(expire(list, 4999)).toHaveLength(1);
  });

  it("returns an empty list unchanged", () => {
    expect(expire([], 1000)).toEqual([]);
  });

  it("preserves kind and text on surviving toasts", () => {
    const list: ToastItem[] = [{ id: 1, kind: "error", text: "boom", createdAt: 100 }];
    expect(expire(list, 200)).toEqual(list);
  });
});
