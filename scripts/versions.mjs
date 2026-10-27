import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { parse } from 'smol-toml';

export function readVersions(root = process.cwd()) {
  const pkg = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'));
  const cargo = parse(readFileSync(join(root, 'Cargo.toml'), 'utf8')).package;
  const entries = parse(readFileSync(join(root, 'Cargo.lock'), 'utf8')).package;
  const locked = entries.filter(entry => entry.name === pkg.name && !entry.source);
  if (cargo.name !== pkg.name || locked.length !== 1) {
    throw new Error('Expected matching package names and exactly one local Cargo.lock package');
  }
  if (!/^\d+\.\d+\.\d+$/.test(pkg.version)) {
    throw new Error(`Only stable release versions are supported: ${pkg.version}`);
  }
  return { name: pkg.name, version: pkg.version, cargo: cargo.version, lock: locked[0].version };
}

export function checkVersions(root = process.cwd()) {
  const versions = readVersions(root);
  if (versions.version !== versions.cargo || versions.version !== versions.lock) {
    throw new Error(`Version mismatch: package.json=${versions.version}, Cargo.toml=${versions.cargo}, Cargo.lock=${versions.lock}`);
  }
  return versions;
}

export function syncVersions(root = process.cwd()) {
  const { name, version } = readVersions(root);
  for (const [file, section] of [['Cargo.toml', /^\[package\][\s\S]*?(?=^\[|$(?![\s\S]))/m],
    ['Cargo.lock', /^\[\[package\]\][\s\S]*?(?=^\[\[package\]\]|$(?![\s\S]))/gm]]) {
    const path = join(root, file);
    let updated = 0;
    const content = readFileSync(path, 'utf8').replace(section, block => {
      const entry = file === 'Cargo.toml' ? parse(block).package : parse(block).package[0];
      if (entry.name !== name || entry.source) return block;
      const result = block.replace(/^version\s*=\s*"[^"]*"/m, `version = "${version}"`);
      updated++;
      return result;
    });
    if (updated !== 1) throw new Error(`Could not locate package version in ${file}`);
    writeFileSync(path, content);
  }
  return checkVersions(root);
}
