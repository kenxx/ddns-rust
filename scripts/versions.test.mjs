import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { checkVersions, syncVersions } from './versions.mjs';
import { releasePlan } from './release-plan.mjs';
import { releaseNotes } from './release-notes.mjs';

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), 'ddns-version-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  writeFileSync(join(root, 'package.json'), JSON.stringify({ name: 'ddns-rust', version: '0.2.0' }));
  writeFileSync(join(root, 'Cargo.toml'), '# keep comments\n[package]\nname = "ddns-rust"\nversion = "0.1.1"\n\n[dependencies]\nserde = "1"\n');
  writeFileSync(join(root, 'Cargo.lock'), '# keep lock comments\nversion = 4\n\n[[package]]\nname = "ddns-rust"\nversion = "0.1.1"\ndependencies = ["serde"]\n\n[[package]]\nname = "serde"\nversion = "1.0.0"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\n');
  return root;
}

test('sync updates only the application versions and is idempotent', t => {
  const root = fixture(t);
  assert.throws(() => checkVersions(root), /Version mismatch/);
  syncVersions(root);
  assert.equal(checkVersions(root).version, '0.2.0');
  assert.match(readFileSync(join(root, 'Cargo.toml'), 'utf8'), /^# keep comments/);
  const lock = readFileSync(join(root, 'Cargo.lock'), 'utf8');
  assert.match(lock, /^# keep lock comments\nversion = 4/);
  assert.match(lock, /name = "serde"\nversion = "1.0.0"/);
  syncVersions(root);
  assert.equal(readFileSync(join(root, 'Cargo.lock'), 'utf8'), lock);
});

test('rejects missing or ambiguous local lockfile entries', t => {
  const root = fixture(t);
  writeFileSync(join(root, 'Cargo.lock'), 'version = 4\n[[package]]\nname = "other"\nversion = "0.1.1"\n');
  assert.throws(() => syncVersions(root), /exactly one local/);
});

test('rejects unsupported prereleases and mismatched package names', t => {
  const root = fixture(t);
  writeFileSync(join(root, 'package.json'), JSON.stringify({ name: 'ddns-rust', version: '0.2.0-beta.1' }));
  assert.throws(() => syncVersions(root), /Only stable/);
  writeFileSync(join(root, 'package.json'), JSON.stringify({ name: 'other', version: '0.2.0' }));
  assert.throws(() => syncVersions(root), /matching package names/);
});

test('release planning creates new tags, retries the same commit, and skips later commits', t => {
  const root = fixture(t);
  syncVersions(root);
  const git = (...args) => execFileSync('git', args, { cwd: root, encoding: 'utf8' });
  git('init', '-q', '--initial-branch=main');
  git('add', '.');
  const commit = () => git('-c', 'user.name=Test', '-c', 'user.email=test@example.com', 'commit', '-q', '--allow-empty', '-m', 'test');
  commit();
  assert.deepEqual(releasePlan(root), { tag: 'v0.2.0', create: true });
  git('-c', 'user.name=Test', '-c', 'user.email=test@example.com', 'tag', '-a', 'v0.2.0', '-m', 'release');
  assert.deepEqual(releasePlan(root), { tag: 'v0.2.0', create: false });
  commit();
  assert.deepEqual(releasePlan(root), { tag: '', create: false });
});

test('release notes contain only the selected version', () => {
  const changelog = '# ddns-rust\n\n## 0.2.0\n\n### Minor Changes\n\n- Basic auth\n\n## 0.1.1\n\n- Older release\n';
  assert.equal(releaseNotes(changelog, '0.2.0'), '## 0.2.0\n\n### Minor Changes\n\n- Basic auth\n');
  assert.throws(() => releaseNotes(changelog, '9.9.9'), /Missing CHANGELOG/);
});
