### PostgreSQL bootstrap

`factstr-postgres` now provides an explicit bootstrap path for local and reference implementations.

`PostgresStore::connect(database_url)` keeps its existing behavior: the target PostgreSQL database must already exist, and FACTSTR initializes the schema it owns inside that database.

`PostgresStore::bootstrap(options)` starts from an existing PostgreSQL server connection, creates the target database when missing, and then returns a ready store through the normal `connect` path.

FACTSTR can create the target PostgreSQL database only through the explicit bootstrap path. FACTSTR still does not provision or run PostgreSQL itself. Bootstrap requires an existing PostgreSQL server and credentials with permission to inspect `pg_database` and create the target database.

Bootstrap database names are intentionally limited to simple identifier-style names matching `[A-Za-z_][A-Za-z0-9_]*`.
