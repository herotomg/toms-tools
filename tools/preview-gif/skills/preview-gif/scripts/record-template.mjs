/**
 * Recorder template. Copy to /tmp, edit the marked sections, run with node.
 *
 * Two passes share one browser context:
 *   1. setup  - navigate/collapse/dismiss so the app is in its opening state.
 *               Its video is discarded. localStorage persists to pass 2.
 *   2. take   - the recorded choreography. Its video is the deliverable.
 *
 * Everything else (cursor, chromium resolution, glide helpers) comes from
 * demo-cursor.mjs next to this file.
 */
import { chromium } from "/Users/tomg/.npm/_npx/9833c18b2d85bc59/node_modules/playwright/index.mjs";
import { rmSync, mkdirSync } from "node:fs";
import {
  cursorInitScript,
  findChromium,
  glideClick,
  glideTo,
  glideType,
} from "/Users/tomg/.claude/skills/preview-gif/scripts/demo-cursor.mjs";

const OUT = "/tmp/preview-video";
const URL = "http://127.0.0.1:8787/";
const W = 1000;
const H = 660;

rmSync(OUT, { recursive: true, force: true });
mkdirSync(OUT, { recursive: true });

const browser = await chromium.launch({ headless: true, executablePath: findChromium() });
const ctx = await browser.newContext({
  viewport: { width: W, height: H },
  recordVideo: { dir: OUT, size: { width: W, height: H } },
  colorScheme: "dark",
});
await ctx.addInitScript(cursorInitScript);

// ---------------------------------------------------------------- pass 1: setup
const setup = await ctx.newPage();
await setup.goto(URL, { waitUntil: "domcontentloaded" });
// EDIT: get the app into the state the take should open in. Collapse chrome you
// do not want in frame, dismiss onboarding nags, select the right target.
await setup.waitForTimeout(1500);
console.log("  · setup complete");
await setup.close();

// ----------------------------------------------------------------- pass 2: take
const page = await ctx.newPage();
const step = async (label, ms = 900) => {
  console.log("  ·", label);
  await page.waitForTimeout(ms);
};

await page.goto(URL, { waitUntil: "domcontentloaded" });
// EDIT: wait on the first thing you interact with, never a bare timeout.
const first = page.getByRole("button", { name: "EDIT ME" });
await first.waitFor({ timeout: 30000 });

// Bring the cursor on screen from a neutral spot before touching anything.
await page.mouse.move(W * 0.62, H * 0.42, { steps: 2 });
await step("cursor on screen", 1500);

// EDIT: the choreography. glideClick dwells on the target before pressing.
await glideClick(page, first);
await step("first action", 1400);

// Drift away by raw coordinates, never a locator: a locator that fails to match
// blocks for its full timeout and pads dead seconds onto the end of the take.
await page.mouse.move(W * 0.68, H * 0.3, { steps: 26 });
await step("settle", 1000);

const video = page.video();
await ctx.close();
console.log("kept take:", await video.path());
await browser.close();
