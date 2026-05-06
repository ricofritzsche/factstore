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
let nativeModule = null;

if (prebuiltPackage !== null) {
  nativeModule = prebuiltPackage;
} else {
  const packageRoot = __dirname;
  const platformBinary = join(
    packageRoot,
    `factstr-node.${process.platform}-${process.arch}.node`,
  );
  const fallbackBinary = join(packageRoot, 'factstr-node.node');

  if (existsSync(platformBinary)) {
    nativeModule = require(platformBinary);
  } else if (existsSync(fallbackBinary)) {
    nativeModule = require(fallbackBinary);
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

function wrapStreamHandle(handle) {
  if (typeof handle !== 'function') {
    throw new TypeError('stream callback must be a function');
  }

  return (events) => {
    try {
      const result = handle(events);
      return result === false ? false : true;
    } catch (error) {
      console.error('factstr-node stream callback failed', error);
      return false;
    }
  };
}

class FactstrMemoryStore {
  #nativeStore;

  constructor() {
    this.#nativeStore = new nativeModule.FactstrMemoryStore();
  }

  append(events) {
    return this.#nativeStore.append(events);
  }

  query(query) {
    return this.#nativeStore.query(query);
  }

  appendIf(events, query, expectedContextVersion) {
    return this.#nativeStore.appendIf(events, query, expectedContextVersion);
  }

  streamAll(handle) {
    return this.#nativeStore.streamAll(wrapStreamHandle(handle));
  }

  streamTo(query, handle) {
    return this.#nativeStore.streamTo(query, wrapStreamHandle(handle));
  }

  streamAllDurable(durableStream, handle) {
    return this.#nativeStore.streamAllDurable(durableStream, wrapStreamHandle(handle));
  }

  streamToDurable(durableStream, query, handle) {
    return this.#nativeStore.streamToDurable(
      durableStream,
      query,
      wrapStreamHandle(handle),
    );
  }
}

class FactstrSqliteStore {
  #nativeStore;

  constructor(databasePath) {
    this.#nativeStore = new nativeModule.FactstrSqliteStore(databasePath);
  }

  append(events) {
    return this.#nativeStore.append(events);
  }

  query(query) {
    return this.#nativeStore.query(query);
  }

  appendIf(events, query, expectedContextVersion) {
    return this.#nativeStore.appendIf(events, query, expectedContextVersion);
  }

  streamAll(handle) {
    return this.#nativeStore.streamAll(wrapStreamHandle(handle));
  }

  streamTo(query, handle) {
    return this.#nativeStore.streamTo(query, wrapStreamHandle(handle));
  }

  streamAllDurable(durableStream, handle) {
    return this.#nativeStore.streamAllDurable(durableStream, wrapStreamHandle(handle));
  }

  streamToDurable(durableStream, query, handle) {
    return this.#nativeStore.streamToDurable(
      durableStream,
      query,
      wrapStreamHandle(handle),
    );
  }
}

class FactstrPostgresStore {
  #nativeStore;

  constructor(databaseUrl) {
    this.#nativeStore = new nativeModule.FactstrPostgresStore(databaseUrl);
  }

  append(events) {
    return this.#nativeStore.append(events);
  }

  query(query) {
    return this.#nativeStore.query(query);
  }

  appendIf(events, query, expectedContextVersion) {
    return this.#nativeStore.appendIf(events, query, expectedContextVersion);
  }

  streamAll(handle) {
    return this.#nativeStore.streamAll(wrapStreamHandle(handle));
  }

  streamTo(query, handle) {
    return this.#nativeStore.streamTo(query, wrapStreamHandle(handle));
  }

  streamAllDurable(durableStream, handle) {
    return this.#nativeStore.streamAllDurable(durableStream, wrapStreamHandle(handle));
  }

  streamToDurable(durableStream, query, handle) {
    return this.#nativeStore.streamToDurable(
      durableStream,
      query,
      wrapStreamHandle(handle),
    );
  }
}

module.exports = {
  ...nativeModule,
  FactstrMemoryStore,
  FactstrPostgresStore,
  FactstrSqliteStore,
};
