import { copyFileSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

import {
  currentPrebuiltTarget,
  prebuiltPackageDirectory,
  prebuiltTargetFromRustTarget,
} from './prebuilt-target.mjs';

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const packageRoot = dirname(scriptDirectory);
const workspaceRoot = dirname(packageRoot);

const targetArgument = process.argv[2] ?? process.env.FACTSTR_NODE_PREBUILT_TARGET ?? null;
const target = resolveTarget(targetArgument);

if (target === null) {
  const suffix = targetArgument === null ? `${process.platform}-${process.arch}` : targetArgument;
  throw new Error(`factstr-node does not have a configured prebuilt target for ${suffix}.`);
}

runOrThrow(
  'cargo',
  ['build', '-p', 'factstr-node', '--release', '--target', target.rustTarget],
  workspaceRoot,
);

const sourceBinary = releaseBinaryPath(target.rustTarget);
const targetDirectory = prebuiltPackageDirectory(packageRoot, target);
const targetBinary = join(targetDirectory, target.nodeFileName);

mkdirSync(targetDirectory, { recursive: true });
copyFileSync(sourceBinary, targetBinary);

function resolveTarget(targetArgument) {
  if (targetArgument === null) {
    return currentPrebuiltTarget();
  }

  return prebuiltTargetFromRustTarget(targetArgument);
}

function releaseBinaryPath(rustTarget) {
  if (rustTarget.includes('windows')) {
    return join(
      workspaceRoot,
      'target',
      rustTarget,
      'release',
      'factstr_node.dll',
    );
  }

  if (rustTarget.includes('apple')) {
    return join(
      workspaceRoot,
      'target',
      rustTarget,
      'release',
      'libfactstr_node.dylib',
    );
  }

  return join(
    workspaceRoot,
    'target',
    rustTarget,
    'release',
    'libfactstr_node.so',
  );
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
