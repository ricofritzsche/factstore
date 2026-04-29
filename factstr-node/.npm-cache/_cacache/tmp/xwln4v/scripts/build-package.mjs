import { copyFileSync, existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const packageDir = resolve(scriptDir, '..');
const workspaceDir = resolve(packageDir, '..');

const cargoResult = spawnSync('cargo', ['build', '-p', 'factstr-node'], {
  cwd: workspaceDir,
  stdio: 'inherit',
});

if (cargoResult.status !== 0) {
  process.exit(cargoResult.status ?? 1);
}

const artifactNameByPlatform = {
  darwin: 'libfactstr_node.dylib',
  linux: 'libfactstr_node.so',
  win32: 'factstr_node.dll',
};

const artifactName = artifactNameByPlatform[process.platform];
if (!artifactName) {
  console.error(`Unsupported platform for factstr-node package build: ${process.platform}`);
  process.exit(1);
}

const sourceArtifact = join(workspaceDir, 'target', 'debug', artifactName);
if (!existsSync(sourceArtifact)) {
  console.error(`Missing native artifact after cargo build: ${sourceArtifact}`);
  process.exit(1);
}

const platformTarget = join(
  packageDir,
  `factstr-node.${process.platform}-${process.arch}.node`,
);
const fallbackTarget = join(packageDir, 'factstr-node.node');

copyFileSync(sourceArtifact, platformTarget);
copyFileSync(sourceArtifact, fallbackTarget);

console.log(`Built factstr-node package binary: ${platformTarget}`);
