import { join } from 'node:path';

export const prebuiltTargets = [
  {
    rustTarget: 'aarch64-apple-darwin',
    packageName: 'factstr-node-darwin-arm64',
    packageDirectoryName: 'darwin-arm64',
    os: 'darwin',
    arch: 'arm64',
    nodeFileName: 'factstr-node.darwin-arm64.node',
  },
  {
    rustTarget: 'x86_64-apple-darwin',
    packageName: 'factstr-node-darwin-x64',
    packageDirectoryName: 'darwin-x64',
    os: 'darwin',
    arch: 'x64',
    nodeFileName: 'factstr-node.darwin-x64.node',
  },
  {
    rustTarget: 'x86_64-unknown-linux-gnu',
    packageName: 'factstr-node-linux-x64-gnu',
    packageDirectoryName: 'linux-x64-gnu',
    os: 'linux',
    arch: 'x64',
    nodeFileName: 'factstr-node.linux-x64-gnu.node',
  },
  {
    rustTarget: 'x86_64-pc-windows-msvc',
    packageName: 'factstr-node-win32-x64-msvc',
    packageDirectoryName: 'win32-x64-msvc',
    os: 'win32',
    arch: 'x64',
    nodeFileName: 'factstr-node.win32-x64-msvc.node',
  },
];

export function currentPrebuiltTarget() {
  if (process.platform === 'linux' && !isLinuxGnu()) {
    return null;
  }

  return (
    prebuiltTargets.find(
      (target) =>
        target.os === process.platform && target.arch === process.arch,
    ) ?? null
  );
}

export function prebuiltTargetFromRustTarget(rustTarget) {
  return prebuiltTargets.find((target) => target.rustTarget === rustTarget) ?? null;
}

export function prebuiltPackageDirectory(packageRoot, target) {
  return join(packageRoot, 'npm', target.packageDirectoryName);
}

function isLinuxGnu() {
  if (process.platform !== 'linux') {
    return false;
  }

  if (process.report && typeof process.report.getReport === 'function') {
    const report = process.report.getReport();
    return Boolean(report?.header?.glibcVersionRuntime);
  }

  return true;
}
