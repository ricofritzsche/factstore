const [crateName, targetVersion] = process.argv.slice(2);

if (!crateName || !targetVersion) {
  throw new Error('Usage: node scripts/wait-for-crates-io-version.mjs <crate-name> <version>');
}

const maxAttempts = 60;
const delayMs = 10_000;

for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
  const publishedVersions = await readPublishedVersions(crateName);

  if (publishedVersions.includes(targetVersion)) {
    console.log(`${crateName}@${targetVersion} is now visible on crates.io`);
    process.exit(0);
  }

  console.log(
    `Waiting for ${crateName}@${targetVersion} to appear on crates.io (${attempt}/${maxAttempts})`,
  );

  await new Promise((resolve) => setTimeout(resolve, delayMs));
}

throw new Error(`Timed out waiting for ${crateName}@${targetVersion} on crates.io`);

async function readPublishedVersions(name) {
  const response = await fetch(`https://crates.io/api/v1/crates/${encodeURIComponent(name)}/versions`, {
    headers: {
      'user-agent': 'factstr-rust-publish-wait/1',
    },
  });

  if (response.status === 404) {
    return [];
  }

  if (!response.ok) {
    throw new Error(`Failed to read crates.io versions for ${name}: ${response.status} ${response.statusText}`);
  }

  const body = await response.json();
  if (!Array.isArray(body.versions)) {
    throw new Error(`Unexpected crates.io response for ${name}`);
  }

  return body.versions
    .map((version) => version.num)
    .filter((version) => typeof version === 'string');
}
