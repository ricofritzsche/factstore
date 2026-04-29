'use strict';

const { existsSync } = require('node:fs');
const { join } = require('node:path');

function currentPrebuiltPackageName() {
  if (process.platform === 'darwin' && process.arch === 'arm64') {
    return '@factstr/factstr-node-darwin-arm64';
  }

  if (process.platform === 'darwin' && process.arch === 'x64') {
    return '@factstr/factstr-node-darwin-x64';
  }

  if (process.platform === 'linux' && process.arch === 'x64' && isLinuxGnu()) {
    return '@factstr/factstr-node-linux-x64-gnu';
  }

  if (process.platform === 'win32' && process.arch === 'x64') {
    return '@factstr/factstr-node-win32-x64-msvc';
  }

  return null;
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

function loadPrebuiltPackage() {
  const packageName = currentPrebuiltPackageName();
  if (packageName === null) {
    return null;
  }

  try {
    return require(packageName);
  } catch (error) {
    if (!isMissingPackageError(error, packageName)) {
      throw error;
    }

    return null;
  }
}

function isMissingPackageError(error, packageName) {
  return (
    error &&
    error.code === 'MODULE_NOT_FOUND' &&
    typeof error.message === 'string' &&
    error.message.includes(packageName)
  );
}

const prebuiltPackage = loadPrebuiltPackage();
let exportedModule = null;

if (prebuiltPackage !== null) {
  exportedModule = prebuiltPackage;
} else {
  const packageRoot = __dirname;
  const platformBinary = join(
    packageRoot,
    `factstr-node.${process.platform}-${process.arch}.node`,
  );
  const fallbackBinary = join(packageRoot, 'factstr-node.node');

  if (existsSync(platformBinary)) {
    exportedModule = require(platformBinary);
  } else if (existsSync(fallbackBinary)) {
    exportedModule = require(fallbackBinary);
  } else {
    const packageName = currentPrebuiltPackageName();
    if (packageName !== null) {
      throw new Error(
        `factstr-node could not load the prebuilt package ${packageName}. Install the matching native package or build locally.`,
      );
    }

    throw new Error(
      `factstr-node does not have a prebuilt package for ${process.platform}-${process.arch}.`,
    );
  }
}

module.exports = exportedModule;
