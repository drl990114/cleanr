#!/usr/bin/env node
// Generate a static Cleanr plugin index from plugins/* bundles.
//
// Usage:
//   node .github/scripts/generate-plugin-index.mjs
//   node .github/scripts/generate-plugin-index.mjs --unlocked
//   node .github/scripts/generate-plugin-index.mjs --check
//   node .github/scripts/generate-plugin-index.mjs --base-url https://example.com/plugins

import { execFileSync } from "node:child_process";

const args = process.argv.slice(2);
const forwardedArgs = ["-p", "cleanr-cli", "--", "plugin", "index"];
let locked = true;

for (let index = 0; index < args.length; index += 1) {
  const arg = args[index];
  if (arg === "--unlocked") {
    locked = false;
  } else if (arg === "--check") {
    forwardedArgs.push("--check");
  } else if (arg === "--base-url") {
    const value = args[index + 1];
    if (!value) {
      throw new Error("--base-url requires a value");
    }
    forwardedArgs.push("--base-url", value);
    index += 1;
  } else if (arg === "--plugin-dir") {
    const value = args[index + 1];
    if (!value) {
      throw new Error("--plugin-dir requires a value");
    }
    forwardedArgs.push("--plugin-dir", value);
    index += 1;
  } else if (arg === "--output") {
    const value = args[index + 1];
    if (!value) {
      throw new Error("--output requires a value");
    }
    forwardedArgs.push("--output", value);
    index += 1;
  } else {
    console.error(
      "usage: generate-plugin-index.mjs [--unlocked] [--check] [--base-url URL] [--plugin-dir DIR] [--output PATH]"
    );
    process.exit(1);
  }
}

const forwarded = locked
  ? ["run", "--locked", ...forwardedArgs]
  : ["run", ...forwardedArgs];

execFileSync("cargo", forwarded, { stdio: "inherit" });
