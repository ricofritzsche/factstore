import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

import {
  currentPrebuiltTarget,
  prebuiltPackageDirectory,
  prebuiltTargetFromRustTarget,
} from './prebuilt-target.mjs';

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const packageRoot = dirname(scriptDirectory);

const targetArgument = process.argv[2] ?? process.env.FACTSTR_NODE_PREBUILT_TARGET ?? null;
const target = resolveTarget(targetArgument);

if (target === null) {
  const suffix = targetArgument === null ? `${process.platform}-${process.arch}` : targetArgument;
  throw new Error(`factstr-node does not have a configured prebuilt target for ${suffix}.`);
}

runOrThrow(
  'node',
  ['./scripts/build-prebuilt-package.mjs', target.rustTarget],
  packageRoot,
);

runOrThrow('npm', ['pack'], prebuiltPackageDirectory(packageRoot, target));

function resolveTarget(targetArgument) {
  if (targetArgument === null) {
    return currentPrebuiltTarget();
  }

  return prebuiltTargetFromRustTarget(targetArgument);
}

function runOrThrow(command, args, cwd) {
  const result = spawnSync(command, args, {
    cwd,
    stdio: 'inherit',
  });

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}
