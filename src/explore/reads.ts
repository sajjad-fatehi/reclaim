import { errorCode } from '../scan/contract';

const retryDelayMs = 120;
const maximumAttempts = 40;
export const staleRead = { code: 'stale_read' };

const tails = new WeakMap<object, Promise<unknown>>();

function wait(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

/**
 * Native reads share one 100 ms budget and must be issued one at a time. Reads
 * for the same owner are serialized; `rate_limited` is retried transparently so
 * fast navigation never surfaces it. `isCurrent` lets callers drop a read whose
 * scope was navigated away from before it ran.
 */
export function readThrottled<Value>(
  owner: object,
  read: () => Promise<Value>,
  isCurrent: () => boolean = () => true,
): Promise<Value> {
  async function run(): Promise<Value> {
    for (let attempt = 0; ; attempt += 1) {
      if (!isCurrent()) throw staleRead;
      try {
        return await read();
      } catch (error) {
        if (errorCode(error) !== 'rate_limited' || attempt >= maximumAttempts)
          throw error;
        await wait(retryDelayMs);
      }
    }
  }
  const result = (tails.get(owner) ?? Promise.resolve()).then(run);
  tails.set(
    owner,
    result.catch(() => undefined),
  );
  return result;
}
