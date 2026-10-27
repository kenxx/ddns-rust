import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

export function releaseNotes(changelog, version) {
  const sections = changelog.split(/^## /m).slice(1);
  const section = sections.find(section => section.split(/\r?\n/, 1)[0].trim() === version);
  if (!section) throw new Error(`Missing CHANGELOG section for ${version}`);
  return `## ${section.trim()}\n`;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const { version } = JSON.parse(readFileSync('package.json', 'utf8'));
  process.stdout.write(releaseNotes(readFileSync('CHANGELOG.md', 'utf8'), version));
}
