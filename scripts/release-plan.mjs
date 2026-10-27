import { execFileSync, spawnSync } from 'node:child_process';
import { checkVersions } from './versions.mjs';

export function releasePlan(root = process.cwd()) {
  const { version } = checkVersions(root);
  const tag = `v${version}`;
  const head = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim();
  const exists = spawnSync('git', ['show-ref', '--verify', '--quiet', `refs/tags/${tag}`], { cwd: root });
  if (exists.status === 1) return { tag, create: true };
  if (exists.status !== 0) throw new Error(`Could not inspect ${tag}`);
  const tagged = execFileSync('git', ['rev-parse', `${tag}^{commit}`], { cwd: root, encoding: 'utf8' }).trim();
  // Retry the same release commit; ordinary later commits do not republish.
  return { tag: tagged === head ? tag : '', create: false };
}
