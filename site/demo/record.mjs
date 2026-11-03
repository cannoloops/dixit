// Rigenera le GIF demo da scenes.html: node site/demo/record.mjs
// Chrome headless via DevTools Protocol (nessuna dipendenza), un fotogramma per istante, poi ffmpeg.
import { spawn, execFileSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const chrome = process.env.CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const FPS = 20, PORT = 9333;

const profile = mkdtempSync(join(tmpdir(), "dixit-rec-"));
const proc = spawn(chrome, ["--headless=new", `--remote-debugging-port=${PORT}`, `--user-data-dir=${profile}`,
  "--hide-scrollbars", "--allow-file-access-from-files", "about:blank"], { stdio: "ignore" });

let targets;
for (let i = 0; i < 50 && !targets; i++) {
  await new Promise(r => setTimeout(r, 200));
  targets = await fetch(`http://127.0.0.1:${PORT}/json/list`).then(r => r.json()).catch(() => null);
}
const ws = new WebSocket(targets.find(t => t.type === "page").webSocketDebuggerUrl);
await new Promise(r => ws.addEventListener("open", r, { once: true }));
let id = 0;
const pending = new Map();
ws.addEventListener("message", e => {
  const m = JSON.parse(e.data);
  if (pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); }
});
const send = (method, params = {}) => new Promise((resolve, reject) => {
  pending.set(++id, m => m.error ? reject(new Error(`${method}: ${m.error.message}`)) : resolve(m.result));
  ws.send(JSON.stringify({ id, method, params }));
});
const evaluate = async expr => (await send("Runtime.evaluate", { expression: expr, awaitPromise: true, returnByValue: true })).result.value;

try {
  // scala 2 e poi lanczos a 680: il testo resta nitido anche nella GIF
  await send("Emulation.setDeviceMetricsOverride", { width: 680, height: 440, deviceScaleFactor: 2, mobile: false });
  for (const scene of ["live", "riassunto", "mcp"]) {
    const frames = mkdtempSync(join(tmpdir(), `dixit-${scene}-`));
    await send("Page.navigate", { url: `${pathToFileURL(join(here, "scenes.html"))}?scene=${scene}&record` });
    while (!(await evaluate("document.readyState === 'complete' && typeof render === 'function'").catch(() => false)))
      await new Promise(r => setTimeout(r, 100));
    await evaluate("document.fonts.ready.then(() => true)");
    const duration = await evaluate("DURATION");
    for (let f = 0; f * (1 / FPS) <= duration; f++) {
      await evaluate(`render(${f / FPS})`);
      const { data } = await send("Page.captureScreenshot", { format: "png" });
      writeFileSync(join(frames, `${String(f).padStart(4, "0")}.png`), Buffer.from(data, "base64"));
    }
    const out = join(here, `${scene}.gif`);
    execFileSync("ffmpeg", ["-y", "-loglevel", "error", "-framerate", String(FPS), "-i", join(frames, "%04d.png"), "-vf",
      "scale=680:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle",
      out]);
    rmSync(frames, { recursive: true });
    console.log(out);
  }
} finally {
  ws.close();
  await new Promise(r => { proc.once("exit", r); proc.kill(); });
  rmSync(profile, { recursive: true, force: true });
}
