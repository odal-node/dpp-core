/**
 * Say which vocabulary records are due a reading.
 *
 * Each record in `crates/dpp-vocab/vocabularies/` rests on somebody having read
 * another organisation's publication, and that publication can change without a
 * commit here. A record whose source has moved reads exactly like one whose
 * source has not. This compares what the records say with what is published today
 * and names the records that have fallen behind. It cannot say a record is wrong,
 * only that it is due a reading.
 *
 * Two checks, both against the network, which is why neither is in `just check`:
 *
 *   pins   every `pins` entry is observed again and compared with the value
 *          recorded. A pin is a commit hash, a release tag or a document's
 *          SHA-256, so any difference is drift. This is the check that matters.
 *
 *   urls   every URL named in a record's `source`, and every listed term's IRI,
 *          is fetched. A 404 or 410 fails, because the record cites something
 *          that no longer exists. A redirect is reported and does not fail: the
 *          address moved and the record should say where it went. Anything else
 *          that goes wrong (a refusal, a timeout, a server error) is reported as
 *          unverified, because a flaky network must not make the check cry wolf.
 *          Namespace IRIs are not fetched: a prefix names a vocabulary and is
 *          under no obligation to resolve. A persistent identifier such as a
 *          w3id.org address is meant to redirect, so MOVED there is information,
 *          not a fault. And a client-rendered site can answer 200 for a path that
 *          does not exist, so `ok` shows an address answers, not that the page
 *          is there; the pins are the check that is not fooled this way.
 *
 * Exit status: 1 if a pin drifted or could not be observed, or a URL is gone;
 * otherwise 0.
 *
 * Needs Node 18+ and the GitHub CLI (`gh`), authenticated. Run it through
 * `just vocab-drift`. After reading a source again, update the record's
 * `finding`, `checkedOn` and the pin together; a pin moved on its own only
 * silences the report.
 */
import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { promisify } from "node:util";

const run = promisify(execFile);

const DIR = process.argv.slice(2).find((a) => !a.startsWith("--")) ?? "crates/dpp-vocab/vocabularies";
const SKIP_URLS = process.argv.includes("--pins-only");
const TIMEOUT_MS = 25_000;
const URL_CONCURRENCY = 6;

const records = readdirSync(DIR)
  .filter((name) => name.endsWith(".json"))
  .sort()
  .map((name) => JSON.parse(readFileSync(join(DIR, name), "utf8")));

async function gh(endpoint) {
  const { stdout } = await run("gh", ["api", endpoint], { maxBuffer: 16 * 1024 * 1024 });
  return JSON.parse(stdout);
}

/** The value a pin would record if it were taken now. */
async function observe(pin) {
  switch (pin.kind) {
    case "github-head": {
      const path = pin.path ? `&path=${encodeURIComponent(pin.path)}` : "";
      const commits = await gh(`repos/${pin.target}/commits?per_page=1${path}`);
      if (!Array.isArray(commits) || commits.length === 0) throw new Error("no commit found");
      return commits[0].sha;
    }
    case "github-release":
      return (await gh(`repos/${pin.target}/releases/latest`)).tag_name;
    case "url-sha256": {
      const res = await fetch(pin.target, { redirect: "follow", signal: AbortSignal.timeout(TIMEOUT_MS) });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      return createHash("sha256").update(Buffer.from(await res.arrayBuffer())).digest("hex");
    }
    default:
      throw new Error(`unknown pin kind ${pin.kind}`);
  }
}

const describe = (pin) => `${pin.kind} ${pin.target}${pin.path ? ` [${pin.path}]` : ""}`;
const short = (value) => (value.length > 16 ? `${value.slice(0, 12)}…` : value);

let failed = false;

// ── pins ────────────────────────────────────────────────────────────────────
console.log("== pins");
let pinCount = 0;
let drifted = 0;
for (const record of records) {
  for (const pin of record.pins ?? []) {
    pinCount++;
    try {
      const now = await observe(pin);
      if (now === pin.value) {
        console.log(`ok       ${record.key}  ${describe(pin)}`);
      } else {
        drifted++;
        failed = true;
        console.log(`DRIFT    ${record.key}  ${describe(pin)}`);
        console.log(`           recorded ${short(pin.value)} on ${pin.observedOn}, now ${short(now)}`);
      }
    } catch (error) {
      failed = true;
      console.log(`UNKNOWN  ${record.key}  ${describe(pin)}`);
      console.log(`           could not be observed: ${String(error.stderr || error.message).trim().split("\n")[0]}`);
    }
  }
}
console.log(`${pinCount} pins, ${drifted} drifted`);

// ── urls ────────────────────────────────────────────────────────────────────
if (!SKIP_URLS) {
  console.log("\n== urls");
  const urlPattern = /https?:\/\/[^\s`"'<>)\]]+/g;
  const cited = new Map(); // url -> records that cite it
  const cite = (key, text) => {
    for (const match of (text ?? "").matchAll(urlPattern)) {
      const url = match[0].replace(/[.,;:]+$/, "");
      if (!cited.has(url)) cited.set(url, new Set());
      cited.get(url).add(key);
    }
  };
  for (const record of records) {
    cite(record.key, record.source);
    for (const term of record.terms ?? []) {
      cite(record.key, term.source);
      cite(record.key, term.iri);
    }
  }

  const sameAddress = (a, b) => a.replace(/\/+$/, "") === b.replace(/\/+$/, "");
  async function probe(url) {
    try {
      const res = await fetch(url, { redirect: "follow", signal: AbortSignal.timeout(TIMEOUT_MS) });
      await res.body?.cancel();
      if (res.status === 404 || res.status === 410) return { state: "gone", note: `HTTP ${res.status}` };
      if (!res.ok) return { state: "unverified", note: `HTTP ${res.status}` };
      if (!sameAddress(res.url, url)) return { state: "moved", note: `now ${res.url}` };
      return { state: "ok" };
    } catch (error) {
      return { state: "unverified", note: String(error.cause?.code ?? error.name ?? error) };
    }
  }

  const queue = [...cited.keys()];
  const results = new Map();
  await Promise.all(
    Array.from({ length: URL_CONCURRENCY }, async () => {
      for (let url = queue.shift(); url !== undefined; url = queue.shift()) results.set(url, await probe(url));
    }),
  );

  const counts = { ok: 0, moved: 0, unverified: 0, gone: 0 };
  for (const [url, { state, note }] of [...results].sort(([a], [b]) => a.localeCompare(b))) {
    counts[state]++;
    if (state === "ok") continue;
    if (state === "gone") failed = true;
    const label = { moved: "MOVED", gone: "GONE", unverified: "UNVERIFIED" }[state].padEnd(10);
    console.log(`${label} ${url}  (${[...cited.get(url)].join(", ")})`);
    console.log(`           ${note}`);
  }
  console.log(`${results.size} urls: ${counts.ok} ok, ${counts.moved} moved, ${counts.unverified} unverified, ${counts.gone} gone`);
}

console.log(failed ? "\nSome records are due a reading." : "\nNo pinned source has moved.");
process.exit(failed ? 1 : 0);
