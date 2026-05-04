import { readFileSync, appendFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const workspaceRoot = fileURLToPath(new URL('../', import.meta.url));

const crates = [
  { name: 'factstr', manifest: 'factstr/Cargo.toml', outputKey: 'factstr' },
  { name: 'factstr-memory', manifest: 'factstr-memory/Cargo.toml', outputKey: 'factstr_memory' },
  { name: 'factstr-sqlite', manifest: 'factstr-sqlite/Cargo.toml', outputKey: 'factstr_sqlite' },
  { name: 'factstr-postgres', manifest: 'factstr-postgres/Cargo.toml', outputKey: 'factstr_postgres' },
];

const plan = {
  should_publish_any: false,
  crates: [],
};

for (const crate of crates) {
  const manifestPath = join(workspaceRoot, crate.manifest);
  const version = readCargoTomlVersion(manifestPath);
  const publishedVersions = await readPublishedVersions(crate.name);
  const shouldPublish = !publishedVersions.includes(version);

  plan.crates.push({
    name: crate.name,
    manifest: crate.manifest,
    version,
    should_publish: shouldPublish,
  });
}

plan.should_publish_any = plan.crates.some((crate) => crate.should_publish);

if (process.env.GITHUB_OUTPUT) {
  appendFileSync(process.env.GITHUB_OUTPUT, `should_publish_any=${plan.should_publish_any}\n`);

  for (const crate of plan.crates) {
    const outputKey = crates.find((entry) => entry.name === crate.name)?.outputKey;
    appendFileSync(process.env.GITHUB_OUTPUT, `${outputKey}_version=${crate.version}\n`);
    appendFileSync(
      process.env.GITHUB_OUTPUT,
      `${outputKey}_should_publish=${crate.should_publish}\n`,
    );
  }
}

console.log(JSON.stringify(plan, null, 2));

function readCargoTomlVersion(path) {
  const contents = readFileSync(path, 'utf8');
  const packageSection = contents.match(/\[package\][\s\S]*?(?=\n\[|$)/);
  if (!packageSection) {
    throw new Error(`Missing [package] section in ${path}`);
  }

  const version = packageSection[0].match(/^version\s*=\s*"([^"]+)"$/m);
  if (!version) {
    throw new Error(`Missing package version in ${path}`);
  }

  return version[1];
}

async function readPublishedVersions(crateName) {
  const response = await fetch(`https://crates.io/api/v1/crates/${encodeURIComponent(crateName)}/versions`, {
    headers: {
      'user-agent': 'factstr-rust-publish-plan/1',
    },
  });

  if (response.status === 404) {
    return [];
  }

  if (!response.ok) {
    throw new Error(`Failed to read crates.io versions for ${crateName}: ${response.status} ${response.statusText}`);
  }

  const body = await response.json();
  if (!Array.isArray(body.versions)) {
    throw new Error(`Unexpected crates.io response for ${crateName}`);
  }

  return body.versions
    .map((version) => version.num)
    .filter((version) => typeof version === 'string');
}
