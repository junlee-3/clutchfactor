export type Platform = "mac" | "windows" | "other";

/** iPadOS Safari in desktop mode reports "Macintosh"; touch points tell it
 *  apart from a Mac (a Mac reports 0). Pure so it can be tested. */
export function detectPlatform(userAgent: string, maxTouchPoints = 0): Platform {
  if (/Windows|Win64|Win32/.test(userAgent)) return "windows";
  const looksMac = /Macintosh|Mac OS X/.test(userAgent) && !/iPhone|iPad|iPod/.test(userAgent);
  if (looksMac && maxTouchPoints <= 1) return "mac";
  return "other";
}
