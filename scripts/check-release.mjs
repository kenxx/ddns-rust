import { execFileSync } from 'node:child_process';
import { cpSync, existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, symlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { checkVersions } from './versions.mjs';
import { releaseNotes } from './release-notes.mjs';

export function checkRelease(root = process.cwd()) {
  const { version } = checkVersions(root);
  const pending = readdirSync(join(root, '.changeset'))
    .filter(name => name.endsWith('.md') && name !== 'README.md');
  if (pending.length === 0) {
    // A version PR has already consumed its changesets. CLI v3 exits 1 here.
    releaseNotes(readFileSync(join(root, 'CHANGELOG.md'), 'utf8'), version);
    console.log(`No pending changesets: versions and CHANGELOG for ${version} are valid`);
    return;
  }

  const stage = mkdtempSync(join(tmpdir(), 'ddns-release-check-'));
  try {
    for (const file of ['package.json', 'Cargo.toml', 'Cargo.lock', '.changeset', 'scripts']) {
      cpSync(join(root, file), join(stage, file), { recursive: true });
    }
    if (existsSync(join(root, 'CHANGELOG.md'))) {
      cpSync(join(root, 'CHANGELOG.md'), join(stage, 'CHANGELOG.md'));
    }
    symlinkSync(join(root, 'node_modules'), join(stage, 'node_modules'), 'junction');
    execFileSync('pnpm', ['version-packages'], { cwd: stage, stdio: 'inherit' });
    const next = checkVersions(stage);
    releaseNotes(readFileSync(join(stage, 'CHANGELOG.md'), 'utf8'), next.version);
    console.log(`Release dry run passed: ${version} -> ${next.version}`);
  } finally {
    rmSync(stage, { recursive: true, force: true });
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  checkRelease();
}
