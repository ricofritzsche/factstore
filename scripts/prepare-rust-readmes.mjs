import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const workspaceRoot = fileURLToPath(new URL('../', import.meta.url));
const checkMode = process.argv.includes('--check');

const versions = {
  factstr: readCargoTomlVersion(join(workspaceRoot, 'factstr', 'Cargo.toml')),
  factstrMemory: readCargoTomlVersion(join(workspaceRoot, 'factstr-memory', 'Cargo.toml')),
  factstrSqlite: readCargoTomlVersion(join(workspaceRoot, 'factstr-sqlite', 'Cargo.toml')),
  factstrPostgres: readCargoTomlVersion(join(workspaceRoot, 'factstr-postgres', 'Cargo.toml')),
};

const targets = [
  {
    path: join(workspaceRoot, 'README.md'),
    transform: (contents) =>
      replaceInstallBlock(
        replaceInstallBlock(
          replaceInstallBlock(
            replaceInstallBlock(contents, 'Core contract', [
              '[dependencies]',
              `factstr = "${versions.factstr}"`,
            ]),
            'Memory',
            [
              '[dependencies]',
              `factstr = "${versions.factstr}"`,
              `factstr-memory = "${versions.factstrMemory}"`,
            ],
          ),
          'SQLite',
          [
            '[dependencies]',
            `factstr = "${versions.factstr}"`,
            `factstr-sqlite = "${versions.factstrSqlite}"`,
          ],
        ),
        'PostgreSQL',
        [
          '[dependencies]',
          `factstr = "${versions.factstr}"`,
          `factstr-postgres = "${versions.factstrPostgres}"`,
        ],
      ),
  },
  {
    path: join(workspaceRoot, 'factstr', 'README.md'),
    transform: (contents) =>
      replaceCargoTomlSnippet(contents, [
        '[dependencies]',
        `factstr = "${versions.factstr}"`,
      ]),
  },
];

const drifted = [];

for (const target of targets) {
  const current = readFileSync(target.path, 'utf8');
  const next = target.transform(current);

  if (current !== next) {
    if (checkMode) {
      drifted.push(target.path);
      continue;
    }

    writeFileSync(target.path, next);
    console.log(`Prepared ${relativeToWorkspace(target.path)}`);
  }
}

if (checkMode && drifted.length > 0) {
  console.error('Rust README version drift detected:');
  for (const path of drifted) {
    console.error(`- ${relativeToWorkspace(path)}`);
  }
  process.exit(1);
}

if (checkMode) {
  console.log('Rust README version sync ok');
}

function replaceInstallBlock(contents, heading, lines) {
  const escapedHeading = escapeRegExp(heading);
  const blockPattern = new RegExp(
    `(${escapedHeading}:\\s*\\n\\n\`\`\`toml\\n)([\\s\\S]*?)(\\n\`\`\`)`,
  );
  if (!blockPattern.test(contents)) {
    throw new Error(`Missing install block for ${heading}`);
  }

  return contents.replace(blockPattern, `$1${lines.join('\n')}$3`);
}

function replaceCargoTomlSnippet(contents, lines) {
  const blockPattern = /(```toml\n)([\s\S]*?)(\n```)/;
  if (!blockPattern.test(contents)) {
    throw new Error('Missing Cargo.toml snippet');
  }

  return contents.replace(blockPattern, `$1${lines.join('\n')}$3`);
}

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

function relativeToWorkspace(path) {
  return path.slice(workspaceRoot.length + 1);
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
