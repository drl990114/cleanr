#!/usr/bin/env node
// Build and publish per-platform npm packages, then publish the wrapper package.
// Existing package versions are skipped so a partially completed release can
// be rerun safely.
// Expects binaries to be available under <root>/artifacts/cleanr-<target>/<binary>.
// Usage: node .github/scripts/publish-npm-packages.mjs <version> [--dry-run]

import {
  chmodSync,
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import { execFileSync } from "node:child_process";

const version = process.argv[2];
const dryRun = process.argv[3] === "--dry-run";
if (
  !version ||
  !/^\d+\.\d+\.\d+$/.test(version) ||
  (process.argv[3] && !dryRun) ||
  process.argv.length > 4
) {
  console.error("usage: publish-npm-packages.mjs <version> [--dry-run]");
  process.exit(1);
}

const root = new URL("../..", import.meta.url).pathname;
const platforms = JSON.parse(readFileSync(join(root, "npm/platforms.json"), "utf8"));
const artifactsDir = join(root, "artifacts");
const tmpDir = join(root, ".npm-publish");

rmSync(tmpDir, { recursive: true, force: true });
mkdirSync(tmpDir, { recursive: true });

function packageVersionExists(name) {
  try {
    const output = execFileSync(
      "npm",
      ["view", `${name}@${version}`, "version", "--json"],
      { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }
    );
    return JSON.parse(output) === version;
  } catch (error) {
    const stderr = error.stderr?.toString() ?? "";
    if (stderr.includes("E404") || stderr.includes("is not in this registry")) {
      return false;
    }
    throw error;
  }
}

function publishOrCheck(name, directory) {
  if (dryRun) {
    checkNpmPack(name, directory);
    return;
  }

  if (packageVersionExists(name)) {
    console.log(`skipping ${name}@${version}: already published`);
    return;
  }

  console.log(`publishing ${name}@${version}`);
  const args = ["publish"];
  if (name.startsWith("@")) {
    args.push("--access", "public");
  }
  execFileSync("npm", args, {
    cwd: directory,
    stdio: "inherit",
  });
}

function checkNpmPack(name, directory) {
  console.log(`checking ${name}@${version}`);
  const output = execFileSync("npm", ["pack", "--dry-run", "--json"], {
    cwd: directory,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "inherit"],
  });
  const packs = JSON.parse(output);
  const pack = packs[0];
  if (!pack || !Array.isArray(pack.files)) {
    throw new Error(`npm pack did not report files for ${name}@${version}`);
  }
  return pack;
}

function verifyPackedExecutable(name, pack, binaryName) {
  const path = `bin/${binaryName}`;
  const file = pack.files.find((entry) => entry.path === path);
  if (!file) {
    throw new Error(`${name} npm package is missing ${path}`);
  }

  const mode = Number(file.mode);
  if (!Number.isInteger(mode) || (mode & 0o777) !== 0o755) {
    const actual = Number.isInteger(mode) ? mode.toString(8) : String(file.mode);
    throw new Error(
      `${name} ${path} must be mode 755 in the npm package; got ${actual}`
    );
  }
}

function publishPlatformPackage(p) {
  const pkgName = p.package;
  const pkgDir = join(tmpDir, pkgName.replace(/^@/, "").replaceAll("/", "-"));
  const binDir = join(pkgDir, "bin");
  mkdirSync(binDir, { recursive: true });

  const artifactDir = join(artifactsDir, `cleanr-${p.target}`);
  if (!existsSync(artifactDir)) {
    throw new Error(`missing artifact directory: ${artifactDir}`);
  }

  const src = join(artifactDir, p.binary);
  if (!existsSync(src)) {
    throw new Error(`missing binary for ${p.target}: ${src}`);
  }

  const dst = join(binDir, p.binary);
  cpSync(src, dst, { preserveTimestamps: true });
  if (p.os !== "win32") {
    chmodSync(dst, 0o755);
  }

  const pkg = {
    name: pkgName,
    version,
    description: `Cleanr binary for ${p.os}-${p.cpu}`,
    license: "MIT",
    os: [p.os],
    cpu: [p.cpu],
    files: ["bin"],
    repository: {
      type: "git",
      url: "git+https://github.com/drl990114/cleanr.git",
    },
  };
  writeFileSync(join(pkgDir, "package.json"), JSON.stringify(pkg, null, 2) + "\n");

  const pack = checkNpmPack(pkgName, pkgDir);
  if (p.os !== "win32") {
    verifyPackedExecutable(pkgName, pack, p.binary);
  }
  if (!dryRun) {
    publishOrCheck(pkgName, pkgDir);
  }
}

function publishWrapperPackage() {
  const wrapperDir = join(root, "npm", "cleanr");
  const pkg = JSON.parse(readFileSync(join(wrapperDir, "package.json"), "utf8"));
  if (pkg.version !== version) {
    throw new Error(
      `${pkg.name} has version ${pkg.version}; expected release version ${version}`
    );
  }
  publishOrCheck(pkg.name, wrapperDir);
}

for (const p of platforms) {
  publishPlatformPackage(p);
}

publishWrapperPackage();
