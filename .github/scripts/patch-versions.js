#!/usr/bin/env node
/**
 * Patches the release version across the workspace.
 * Usage: node patch-versions.js <version>
 *
 * semantic-release runs this as its prepare step (see .releaserc.json) and
 * commits the result. The release workflow refuses to package a tag that
 * disagrees with any of these files.
 *
 * Files patched:
 * - Cargo.toml        ([workspace.package] version, inherited by every member)
 * - Cargo.lock        (the entry of every workspace member)
 * - tooth.json        (top-level version, if the file exists)
 * - shim/xmake.lua    (modVersion, if the file exists)
 *
 * Members are read from the root Cargo.toml, so a new crate is picked up
 * without editing this script. A member that sets its own version instead of
 * `version.workspace = true` fails the run, as it would drift from the tag.
 */

const fs = require('fs');
const path = require('path');

const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(version)) {
  console.error('Usage: node patch-versions.js <version>');
  process.exit(1);
}

const rootDir = path.resolve(__dirname, '../..');

function read(filePath) {
  if (!fs.existsSync(filePath)) {
    console.error(`File not found: ${filePath}`);
    process.exit(1);
  }
  return fs.readFileSync(filePath, 'utf8');
}

function replaceOnce(filePath, content, pattern, replacement) {
  if (!pattern.test(content)) {
    console.error(`Version not found in ${filePath}`);
    process.exit(1);
  }
  fs.writeFileSync(filePath, content.replace(pattern, replacement));
  console.log(`Patched: ${path.relative(rootDir, filePath)}`);
}

function memberNames(rootToml) {
  const block = rootToml.match(/^members\s*=\s*\[([^\]]*)\]/m);
  if (!block) {
    console.error('No [workspace] members in Cargo.toml');
    process.exit(1);
  }
  const dirs = [...block[1].matchAll(/"([^"]+)"/g)].map((m) => m[1]);
  return dirs.map((dir) => {
    const manifest = path.join(rootDir, dir, 'Cargo.toml');
    const content = read(manifest);
    if (!/^version\.workspace\s*=\s*true/m.test(content)) {
      console.error(`${dir}/Cargo.toml must use version.workspace = true`);
      process.exit(1);
    }
    return content.match(/^name\s*=\s*"([^"]+)"/m)[1];
  });
}

console.log(`Patching files to version ${version}...`);
console.log('');

const cargoToml = path.join(rootDir, 'Cargo.toml');
const rootToml = read(cargoToml);
const members = memberNames(rootToml);
replaceOnce(
  cargoToml,
  rootToml,
  /(^\[workspace\.package\][^[]*?^version\s*=\s*)"[^"]*"/m,
  `$1"${version}"`,
);

const lockPath = path.join(rootDir, 'Cargo.lock');
if (fs.existsSync(lockPath)) {
  let lock = read(lockPath);
  for (const name of members) {
    const pattern = new RegExp(
      `(\\[\\[package\\]\\]\\r?\\nname = "${name}"\\r?\\nversion = ")[^"]*"`,
    );
    if (!pattern.test(lock)) {
      console.error(`${name} not found in Cargo.lock`);
      process.exit(1);
    }
    lock = lock.replace(pattern, `$1${version}"`);
  }
  fs.writeFileSync(lockPath, lock);
  console.log(`Patched: Cargo.lock (${members.join(', ')})`);
}

const toothPath = path.join(rootDir, 'tooth.json');
if (fs.existsSync(toothPath)) {
  replaceOnce(
    toothPath,
    read(toothPath),
    /^(  "version":\s*)"[^"]*"/m,
    `$1"${version}"`,
  );
}

const xmakePath = path.join(rootDir, 'shim', 'xmake.lua');
if (fs.existsSync(xmakePath)) {
  replaceOnce(
    xmakePath,
    read(xmakePath),
    /(modVersion\s*=\s*)"[^"]*"/,
    `$1"${version}"`,
  );
}

console.log('');
console.log(`All files patched to version ${version}`);
