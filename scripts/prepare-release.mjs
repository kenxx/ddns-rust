import { execFileSync } from 'node:child_process';
import { appendFileSync } from 'node:fs';
import { releasePlan } from './release-plan.mjs';

if (!process.env.GITHUB_ACTIONS || !process.env.GITHUB_OUTPUT) {
  throw new Error('This command creates release tags and must run in GitHub Actions');
}
const { tag, create } = releasePlan();
if (tag) {
  if (create) execFileSync('git', ['tag', tag], { stdio: 'inherit' });
  execFileSync('git', ['push', 'origin', `refs/tags/${tag}`], { stdio: 'inherit' });
  console.log(`Release ${tag}`);
} else {
  console.log('The current version is already tagged on an earlier commit; nothing to release');
}
appendFileSync(process.env.GITHUB_OUTPUT, `tag=${tag}\n`);
