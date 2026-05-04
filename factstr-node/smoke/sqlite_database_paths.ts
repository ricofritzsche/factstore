declare const process: {
  on(event: string, listener: () => void): void;
};
declare function require(moduleName: string): any;

const { mkdtempSync, rmSync } = require('node:fs');
const { join } = require('node:path');
const { tmpdir } = require('node:os');

type SqliteDatabasePaths = {
  databasePath(name: string): string;
};

export function createSqliteDatabasePaths(prefix: string): SqliteDatabasePaths {
  const directory = mkdtempSync(join(tmpdir(), prefix));

  let cleanedUp = false;
  const cleanup = (): void => {
    if (cleanedUp) {
      return;
    }

    cleanedUp = true;
    rmSync(directory, { recursive: true, force: true });
  };

  process.on('exit', cleanup);

  return {
    databasePath(name: string): string {
      return join(directory, `${name}.sqlite`);
    },
  };
}
