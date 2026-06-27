#!/usr/bin/env node

"use strict";

const { spawnSync } = require("node:child_process");

const packages = {
  "darwin-arm64": ["@cleanr-cli/darwin-arm64", "cleanr"],
  "darwin-x64": ["@cleanr-cli/darwin-x64", "cleanr"],
  "linux-arm": ["@cleanr-cli/linux-arm", "cleanr"],
  "linux-arm64": ["@cleanr-cli/linux-arm64", "cleanr"],
  "linux-x64": ["@cleanr-cli/linux-x64", "cleanr"],
  "win32-ia32": ["@cleanr-cli/win32-ia32", "cleanr.exe"],
  "win32-x64": ["@cleanr-cli/win32-x64", "cleanr.exe"]
};

const platformKey = `${process.platform}-${process.arch}`;
const packageInfo = packages[platformKey];

if (!packageInfo) {
  console.error(`cleanr does not provide a binary for ${platformKey}`);
  process.exit(1);
}

const [packageName, binaryName] = packageInfo;
let binaryPath;

try {
  binaryPath = require.resolve(`${packageName}/bin/${binaryName}`);
} catch (error) {
  console.error(
    `The optional package ${packageName} is missing. ` +
      "Reinstall without --no-optional, or use a GitHub Release binary."
  );
  process.exit(1);
}

const result = spawnSync(binaryPath, process.argv.slice(2), { stdio: "inherit" });

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

if (result.signal) {
  process.kill(process.pid, result.signal);
}

process.exit(result.status ?? 1);
