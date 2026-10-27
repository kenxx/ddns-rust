import { checkVersions } from './versions.mjs';

const { name, version } = checkVersions();
console.log(`${name}: package.json, Cargo.toml and Cargo.lock agree on ${version}`);
