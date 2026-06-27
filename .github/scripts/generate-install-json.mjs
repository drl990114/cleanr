#!/usr/bin/env node
// Generate install.json for a GitHub release from npm/platforms.json.
// Usage: node .github/scripts/generate-install-json.mjs <version>

import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+$/.test(version)) {
  console.error("usage: generate-install-json.mjs <version>");
  process.exit(1);
}

const root = new URL("../..", import.meta.url).pathname;
const platforms = JSON.parse(readFileSync(join(root, "npm/platforms.json"), "utf8"));

const releaseUrl = `https://github.com/drl990114/cleanr/releases/tag/v${version}`;
const downloadBase = `https://github.com/drl990114/cleanr/releases/download/v${version}`;

const output = {
  version: `v${version}`,
  notes: `Cleanr v${version}`,
  pub_date: new Date().toISOString(),
  release_url: releaseUrl,
  platforms: {},
};

for (const p of platforms) {
  const key = `${p.os}-${p.cpu}`;
  const ext = p.binary.endsWith(".exe") ? ".exe" : "";
  const filename = `cleanr-${p.target}${ext}`;
  output.platforms[key] = {
    url: `${downloadBase}/${filename}`,
  };
}

const outPath = join(root, "install.json");
writeFileSync(outPath, JSON.stringify(output, null, 2) + "\n");
console.log(`generated ${outPath}`);
