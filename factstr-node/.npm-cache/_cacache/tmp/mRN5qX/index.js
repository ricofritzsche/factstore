'use strict';

const { existsSync } = require('node:fs');
const { join } = require('node:path');

const packageRoot = __dirname;
const platformBinary = join(
  packageRoot,
  `factstr-node.${process.platform}-${process.arch}.node`,
);
const fallbackBinary = join(packageRoot, 'factstr-node.node');

const nativeModulePath = existsSync(platformBinary) ? platformBinary : fallbackBinary;

module.exports = require(nativeModulePath);
