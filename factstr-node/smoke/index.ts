import { runAppendQueryAppendIfSmoke } from './append_query_append_if';
import { runDirectNativeDurableFailureSmoke } from './direct_native_durable_failure';
import { runLiveStreamsSmoke } from './live_streams';
import { runMemoryDurableStreamsSmoke } from './memory_durable_streams';
import { runSqliteDurableStreamsSmoke } from './sqlite_durable_streams';

declare const process: {
  exitCode?: number;
};

async function main(): Promise<void> {
  await runAppendQueryAppendIfSmoke();
  await runLiveStreamsSmoke();
  await runMemoryDurableStreamsSmoke();
  await runSqliteDurableStreamsSmoke();
  await runDirectNativeDurableFailureSmoke();
  console.log('factstr-node TypeScript smoke test passed');
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
