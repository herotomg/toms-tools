/**
 * Synthetic cursor overlay for Playwright video recordings.
 *
 * Playwright's video is the compositor surface, which never contains the OS
 * cursor, and CDP input events never move the real one. So draw our own: an
 * arrow that follows the interpolated `mousemove` events `page.mouse.move(...,
 * {steps})` emits, plus a ripple on mousedown so a click is visible.
 *
 * Injected via context.addInitScript so it survives SPA navigation.
 */
import { existsSync, readdirSync } from "node:fs";

export const cursorInitScript = () => {
  if (window.__demoCursorInstalled) return;
  window.__demoCursorInstalled = true;

  const install = () => {
    const root = document.documentElement;

    const arrow = document.createElement("div");
    arrow.style.cssText =
      "position:fixed;top:0;left:0;width:22px;height:26px;pointer-events:none;" +
      "z-index:2147483647;opacity:0;will-change:transform;" +
      "transition:transform 45ms linear,opacity 160ms ease-out";
    arrow.innerHTML =
      '<svg width="22" height="26" viewBox="0 0 22 26" xmlns="http://www.w3.org/2000/svg"' +
      ' style="filter:drop-shadow(0 1px 3px rgba(0,0,0,.55))">' +
      '<path d="M3 2.2 L3 20.4 L7.7 15.9 L10.7 22.9 L13.6 21.7 L10.6 14.8 L17.2 14.4 Z"' +
      ' fill="#ffffff" stroke="#111111" stroke-width="1.3" stroke-linejoin="round"/></svg>';

    const ripples = document.createElement("div");
    ripples.style.cssText =
      "position:fixed;top:0;left:0;width:0;height:0;pointer-events:none;z-index:2147483646";

    root.appendChild(ripples);
    root.appendChild(arrow);

    let x = -100;
    let y = -100;
    const place = (scale) =>
      (arrow.style.transform = `translate(${x}px,${y}px)${scale ? ` scale(${scale})` : ""}`);

    addEventListener(
      "mousemove",
      (e) => {
        x = e.clientX;
        y = e.clientY;
        arrow.style.opacity = "1";
        place();
      },
      true,
    );

    addEventListener(
      "mousedown",
      () => {
        place(0.85);
        const r = document.createElement("div");
        r.style.cssText =
          `position:fixed;left:${x}px;top:${y}px;width:10px;height:10px;` +
          "margin:-5px 0 0 -5px;border-radius:50%;background:rgba(52,211,153,.55);" +
          "box-shadow:0 0 0 2px rgba(52,211,153,.9);pointer-events:none;" +
          "transition:transform 430ms ease-out,opacity 430ms ease-out";
        ripples.appendChild(r);
        requestAnimationFrame(() => {
          r.style.transform = "scale(3.6)";
          r.style.opacity = "0";
        });
        setTimeout(() => r.remove(), 470);
      },
      true,
    );

    addEventListener("mouseup", () => place(), true);
    place();
  };

  if (document.documentElement) install();
  else addEventListener("DOMContentLoaded", install, { once: true });
};

/** Glides to a locator's centre, so the click is preceded by visible travel. */
export async function glideTo(page, locator, steps = 24) {
  const box = await locator.boundingBox();
  if (!box) throw new Error("glideTo: locator has no bounding box");
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2, { steps });
  return box;
}

/**
 * locator.click() teleports, so always glide first, pause, then press.
 *
 * `settle` is what makes the click legible: the pointer rests on the target
 * long enough for a viewer to read it (and for the hover state to show) before
 * anything happens. Below ~500ms it reads as an instant jump at 20fps.
 */
export async function glideClick(page, locator, { steps = 24, settle = 650 } = {}) {
  await glideTo(page, locator, steps);
  await page.waitForTimeout(settle);
  await locator.click();
}

/**
 * Resolves a Chromium that actually exists on disk.
 *
 * `npx playwright` often expects a browser revision newer than whatever is in
 * the ms-playwright cache, and then `chromium.launch()` fails telling you to run
 * `npx playwright install`. Rather than download a second browser, point
 * executablePath at the newest cached build.
 */
export function findChromium() {
  const base = `${process.env.HOME}/Library/Caches/ms-playwright`;
  const builds = readdirSync(base)
    .filter((d) => /^chromium-\d+$/.test(d))
    .sort((a, b) => Number(b.split("-")[1]) - Number(a.split("-")[1]));
  for (const build of builds) {
    for (const rel of [
      "chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
      "chrome-mac/Chromium.app/Contents/MacOS/Chromium",
    ]) {
      const candidate = `${base}/${build}/${rel}`;
      if (existsSync(candidate)) return candidate;
    }
  }
  throw new Error(`No cached Chromium under ${base}; run: npx playwright install chromium`);
}

/** Types into a locator at a human cadence, after gliding to it. */
export async function glideType(page, locator, text, { delay = 55, settle = 650 } = {}) {
  await glideClick(page, locator, { settle });
  await locator.pressSequentially(text, { delay });
}
