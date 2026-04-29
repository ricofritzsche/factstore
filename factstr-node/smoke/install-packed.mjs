import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

import {
  currentPrebuiltTarget,
  npmPackFilename,
  prebuiltPackageDirectory,
} from '../scripts/prebuilt-target.mjs';

const smokeDirectory = dirname(fileURLToPath(import.meta.url));
const packageRoot = dirname(smokeDirectory);
const target = currentPrebuiltTarget();
const mainVersion = readPackageJsonVersion(join(packageRoot, 'package.json'));

if (target === null) {
  throw new Error(
    `factstr-node does not have a configured packed smoke target for ${process.platform}-${process.arch}.`,
  );
}

const prebuiltVersion = readPackageJsonVersion(
  join(prebuiltPackageDirectory(packageRoot, target), 'package.json'),
);

const mainTarball = join(
  packageRoot,
  npmPackFilename('@factstr/factstr-node', mainVersion),
);
const prebuiltTarball = join(
  prebuiltPackageDirectory(packageRoot, target),
  npmPackFilename(target.packageName, prebuiltVersion),
);

const result = spawnSync(
  'npm',
  [
    'install',
    '--offline',
    '--no-package-lock',
    '--no-save',
    mainTarball,
    prebuiltTarball,
  ],
  {
    cwd: smokeDirectory,
    stdio: 'inherit',
  },
);

if (result.status !== 0) {
  process.exit(result.status ?? 1);
}

function readPackageJsonVersion(path) {
  return JSON.parse(readFileSync(path, 'utf8')).version;
}
