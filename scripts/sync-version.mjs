import { syncVersions } from './versions.mjs';

const { version } = syncVersions();
console.log(`Synchronized Cargo.toml and Cargo.lock to ${version}`);
