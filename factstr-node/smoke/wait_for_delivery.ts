function wait(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

export async function waitForCondition(
  description: string,
  condition: () => boolean,
  timeoutMs = 500,
  pollIntervalMs = 10,
): Promise<void> {
  const startedAt = Date.now();

  while (Date.now() - startedAt < timeoutMs) {
    if (condition()) {
      return;
    }

    await wait(pollIntervalMs);
  }

  throw new Error(`timed out waiting for ${description}`);
}

export async function waitForNoNewDeliveries(
  description: string,
  currentCount: () => number,
  expectedCount: number,
  stableMs = 100,
  timeoutMs = 400,
  pollIntervalMs = 10,
): Promise<void> {
  const startedAt = Date.now();
  let stableStartedAt: number | null = null;

  while (Date.now() - startedAt < timeoutMs) {
    const count = currentCount();
    if (count !== expectedCount) {
      throw new Error(
        `${description} changed unexpectedly: expected ${expectedCount}, got ${count}`,
      );
    }

    if (stableStartedAt === null) {
      stableStartedAt = Date.now();
    }

    if (Date.now() - stableStartedAt >= stableMs) {
      return;
    }

    await wait(pollIntervalMs);
  }

  throw new Error(`timed out waiting for stable ${description}`);
}
