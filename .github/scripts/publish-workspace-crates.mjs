#!/usr/bin/env node
// Publish all unpublished crates in the workspace with Cargo's native
// multi-package dependency ordering.
//
// Usage:
//   node .github/scripts/publish-workspace-crates.mjs <version>
//   node .github/scripts/publish-workspace-crates.mjs <version> --dry-run --allow-dirty

import { execFileSync } from "node:child_process";

const version = process.argv[2];
const flags = new Set(process.argv.slice(3));
const supportedFlags = new Set(["--dry-run", "--allow-dirty"]);

if (
  !version ||
  !/^\d+\.\d+\.\d+$/.test(version) ||
  [...flags].some((flag) => !supportedFlags.has(flag))
) {
  console.error(
    "usage: publish-workspace-crates.mjs <version> [--dry-run] [--allow-dirty]"
  );
  process.exit(1);
}

const root = new URL("../..", import.meta.url).pathname;
const metadata = JSON.parse(
  execFileSync(
    "cargo",
    ["metadata", "--no-deps", "--format-version", "1"],
    { cwd: root, encoding: "utf8" }
  )
);

const packages = metadata.packages.filter(
  (pkg) => pkg.publish === null || pkg.publish.includes("crates-io")
);

for (const pkg of packages) {
  if (pkg.version !== version) {
    throw new Error(
      `${pkg.name} has version ${pkg.version}; expected release version ${version}`
    );
  }
}

async function crateVersionExists(name) {
  const url =
    `https://crates.io/api/v1/crates/${encodeURIComponent(name)}/` +
    encodeURIComponent(version);

  for (let attempt = 1; attempt <= 3; attempt += 1) {
    const response = await fetch(url, {
      headers: {
        Accept: "application/json",
        "User-Agent": "cleanr-release-workflow/1.0",
      },
    });

    if (response.ok) {
      return true;
    }
    if (response.status === 404) {
      return false;
    }
    if (attempt === 3 || (response.status < 500 && response.status !== 429)) {
      throw new Error(
        `unable to query ${name}@${version} on crates.io: HTTP ${response.status}`
      );
    }

    await new Promise((resolve) => setTimeout(resolve, attempt * 2_000));
  }

  return false;
}

const unpublished = [];

if (flags.has("--dry-run")) {
  for (const pkg of packages) {
    unpublished.push(pkg.name);
  }
  console.log("dry-run: validating every local workspace crate");
} else {
  for (const pkg of packages) {
    if (await crateVersionExists(pkg.name)) {
      console.log(`skipping ${pkg.name}@${version}: already published`);
    } else {
      unpublished.push(pkg.name);
    }
  }
}

if (unpublished.length === 0) {
  console.log(`all workspace crates at ${version} are already published`);
  process.exit(0);
}

console.log(`publishing workspace crates: ${unpublished.join(", ")}`);

const args = ["publish", "--locked"];
for (const name of unpublished) {
  args.push("--package", name);
}
if (flags.has("--dry-run")) {
  args.push("--dry-run");
}
if (flags.has("--allow-dirty")) {
  args.push("--allow-dirty");
}

execFileSync("cargo", args, {
  cwd: root,
  stdio: "inherit",
});
