import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

const source = resolve(dirname(fileURLToPath(import.meta.url)), '..');

test('restores dependencies after checkout cleanup and prepares tags in an isolated remote', t => {
  const temp = mkdtempSync(join(tmpdir(), 'ddns-release-bootstrap-'));
  t.after(() => rmSync(temp, { recursive: true, force: true }));
  const root = join(temp, 'checkout');
  const remote = join(temp, 'remote.git');
  mkdirSync(root);
  for (const file of ['package.json', 'pnpm-lock.yaml', 'Cargo.toml', 'Cargo.lock', 'scripts', '.gitignore']) {
    cpSync(join(source, file), join(root, file), { recursive: true });
  }
  const run = (command, args, options = {}) => execFileSync(command, args, { cwd: root, encoding: 'utf8', ...options });
  run('git', ['init', '-q', '--initial-branch=main']);
  run('git', ['init', '-q', '--bare', remote]);
  run('git', ['remote', 'add', 'origin', remote]);
  run('git', ['add', '.']);
  const commit = () => run('git', ['-c', 'user.name=Test', '-c', 'user.email=test@example.com', 'commit', '-q', '--allow-empty', '-m', 'release fixture']);
  commit();
  const install = () => run('pnpm', ['install', '--frozen-lockfile', '--offline', '--ignore-scripts']);
  install();
  run('node', ['scripts/check-versions.mjs']);

  // actions/checkout's default cleanup removes installed, ignored dependencies.
  run('git', ['clean', '-ffdx']);
  const missing = spawnSync('node', ['scripts/check-versions.mjs'], { cwd: root, encoding: 'utf8' });
  assert.notEqual(missing.status, 0);
  assert.match(missing.stderr, /Cannot find package 'smol-toml'/);
  install();

  const { version } = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'));
  const output = join(temp, 'github-output');
  const prepare = () => run('node', ['scripts/prepare-release.mjs'], {
    env: { ...process.env, GITHUB_ACTIONS: 'true', GITHUB_OUTPUT: output },
  });
  const head = run('git', ['rev-parse', 'HEAD']).trim();
  prepare();
  assert.equal(run('git', ['--git-dir', remote, 'rev-parse', `refs/tags/v${version}`]).trim(), head);
  assert.equal(readFileSync(output, 'utf8'), `tag=v${version}\n`);
  prepare();
  commit();
  prepare();
  assert.equal(run('git', ['--git-dir', remote, 'rev-parse', `refs/tags/v${version}`]).trim(), head);
  assert.ok(readFileSync(output, 'utf8').endsWith('tag=\n'));
});
