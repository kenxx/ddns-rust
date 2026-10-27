import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { cpSync, mkdtempSync, readFileSync, readdirSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { checkRelease } from './check-release.mjs';

const source = resolve(dirname(fileURLToPath(import.meta.url)), '..');

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), 'ddns-release-fixture-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  for (const file of ['package.json', 'Cargo.toml', 'Cargo.lock', '.changeset', 'scripts']) {
    cpSync(join(source, file), join(root, file), { recursive: true });
  }
  // Make this test independent of the repository's current pending changesets.
  for (const file of readdirSync(join(root, '.changeset'))) {
    if (file.endsWith('.md') && file !== 'README.md') rmSync(join(root, '.changeset', file));
  }
  symlinkSync(join(source, 'node_modules'), join(root, 'node_modules'), 'junction');
  return root;
}

test('checks both an unreleased change and the consumed version PR without modifying either', t => {
  const root = fixture(t);
  const change = join(root, '.changeset', 'test.md');
  writeFileSync(change, '---\n"ddns-rust": patch\n---\n\nRelease check regression test.\n');
  const before = readFileSync(join(root, 'package.json'), 'utf8');
  checkRelease(root);
  assert.equal(readFileSync(join(root, 'package.json'), 'utf8'), before);
  assert.match(readFileSync(change, 'utf8'), /regression test/);

  execFileSync('pnpm', ['version-packages'], { cwd: root, stdio: 'pipe' });
  const consumed = readFileSync(join(root, 'package.json'), 'utf8');
  assert.notEqual(consumed, before);
  checkRelease(root);
  assert.equal(readFileSync(join(root, 'package.json'), 'utf8'), consumed);
});

test('consumed changesets still require valid release notes', t => {
  const root = fixture(t);
  assert.throws(() => checkRelease(root), /ENOENT/);
  writeFileSync(join(root, 'CHANGELOG.md'), '# ddns-rust\n\n## 999.0.0\n\nWrong version\n');
  assert.throws(() => checkRelease(root), /Missing CHANGELOG/);
});
