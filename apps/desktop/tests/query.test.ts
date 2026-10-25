import { describe, expect, it, vi } from 'vitest';
import { QueryRunner } from '../src/lib/query';
import type { Correction, Suggestion } from '../src/lib/types';

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(done => { resolve = done; });
  return { promise, resolve };
}
const suggestion = (form: string): Suggestion => ({ form, kind: 'headword', entry: form, headword: form, homograph: null, anchor: 0 });
function backend() {
  return { suggest: vi.fn<(...args: [string, number]) => Promise<Suggestion[]>>(), correct: vi.fn(async () => [] as Correction[]), search: vi.fn(async () => []) };
}

describe('query ordering', () => {
  it('discards an old suggestion that resolves after the new query', async () => {
    const api = backend();
    const old = deferred<Suggestion[]>();
    const fresh = deferred<Suggestion[]>();
    api.suggest.mockReturnValueOnce(old.promise).mockReturnValueOnce(fresh.promise);
    const queries = new QueryRunner(api);
    const a = queries.run('mo', false, null);
    const b = queries.run('moon', false, null);
    fresh.resolve([suggestion('moonjar')]);
    expect((await b)?.rows[0].headword).toBe('moonjar');
    old.resolve([suggestion('old')]);
    expect(await a).toBeNull();
    expect(api.suggest).toHaveBeenLastCalledWith('moon', 30);
  });

  it('discards a correction if input changes while it is pending', async () => {
    const api = backend();
    const correction = deferred<Correction[]>();
    api.suggest.mockResolvedValueOnce([]).mockResolvedValueOnce([suggestion('fernlight')]);
    api.correct.mockReturnValueOnce(correction.promise);
    const queries = new QueryRunner(api);
    const a = queries.run('monjar', false, null);
    await vi.waitFor(() => expect(api.correct).toHaveBeenCalledWith('monjar', 10));
    const b = queries.run('fern', false, null);
    expect((await b)?.corrected).toBe(false);
    correction.resolve([{ ...suggestion('moonjar'), distance: 1 }]);
    expect(await a).toBeNull();
  });

  it('only corrects an empty result with at least three letters', async () => {
    const api = backend();
    api.suggest.mockResolvedValue([]);
    api.correct.mockResolvedValue([{ ...suggestion('moonjar'), distance: 1 }]);
    const queries = new QueryRunner(api);
    await queries.run('12a', false, null);
    await queries.run('', false, null);
    expect(api.correct).not.toHaveBeenCalled();
    expect((await queries.run('monjar', false, null))?.corrected).toBe(true);
  });

  it('discards an old full-text response and passes the column filter', async () => {
    const api = backend();
    api.suggest.mockResolvedValue([]);
    const old = deferred<never[]>();
    api.search.mockReturnValueOnce(old.promise).mockResolvedValueOnce([]);
    const queries = new QueryRunner(api);
    const a = queries.run('quiet', true, 'text');
    await queries.run('leaf', true, 'example');
    old.resolve([]);
    expect(await a).toBeNull();
    expect(api.search).toHaveBeenLastCalledWith('leaf', 30, 'example');
    expect(api.correct).not.toHaveBeenCalled();
  });

  it('invalidates a pending query when the pack changes', async () => {
    const api = backend();
    const pending = deferred<Suggestion[]>();
    api.suggest.mockReturnValueOnce(pending.promise);
    const queries = new QueryRunner(api);
    const result = queries.run('moon', false, null);
    queries.cancel();
    pending.resolve([suggestion('moonjar')]);
    expect(await result).toBeNull();
  });
});
