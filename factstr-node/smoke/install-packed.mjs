import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

import {
  currentPrebuiltTarget,
  prebuiltPackageDirectory,
} from '../scripts/prebuilt-target.mjs';

const smokeDirectory = dirname(fileURLToPath(import.meta.url));
const packageRoot = dirname(smokeDirectory);
const target = currentPrebuiltTarget();

if (target === null) {
  throw new Error(
    `factstr-node does not have a configured packed smoke target for ${process.platform}-${process.arch}.`,
  );
}

const mainTarball = join(packageRoot, 'factstr-node-0.1.0.tgz');
const prebuiltTarball = join(
  prebuiltPackageDirectory(packageRoot, target),
  `${target.packageName}-0.1.0.tgz`,
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
