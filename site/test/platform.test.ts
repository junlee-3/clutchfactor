import { describe, expect, it } from "vitest";
import { detectPlatform } from "../src/platform";

const UA = {
  win: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36",
  mac: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36",
  ipadDesktopMode:
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Safari/605.1.15",
  iphone:
    "Mozilla/5.0 (iPhone; CPU iPhone OS 17_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Mobile/15E148 Safari/604.1",
  linux: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36",
  android: "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Mobile Safari/537.36",
};

describe("detectPlatform", () => {
  it("Windows", () => expect(detectPlatform(UA.win)).toBe("windows"));
  it("macOS", () => expect(detectPlatform(UA.mac, 0)).toBe("mac"));
  it("iPadOS in desktop mode reports Macintosh but has touch points → other", () =>
    expect(detectPlatform(UA.ipadDesktopMode, 5)).toBe("other"));
  it("iPhone → other", () => expect(detectPlatform(UA.iphone, 5)).toBe("other"));
  it("Linux and Android → other", () => {
    expect(detectPlatform(UA.linux)).toBe("other");
    expect(detectPlatform(UA.android)).toBe("other");
  });
});
