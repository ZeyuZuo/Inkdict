// Benchmark mode (src-tauri/src/bench.rs): type queries key by key, open
// entries, and report the input-to-paint times the controller measures.
import { invoke } from '@tauri-apps/api/core';
import { isTauri } from './api';
import { summarize } from './perf';
import type { Dictionary } from './controller.svelte';

interface Plan { queries: string[]; entries: string[] }

const frame = () => new Promise<void>(resolve => requestAnimationFrame(() => resolve()));

/** Wait (up to ~1 s) for the next measurement after `version`; the sample
 *  arrays keep only the last 200, so their length cannot be the signal. */
async function nextSample(dictionary: Dictionary, samples: number[], version: number): Promise<number | undefined> {
  for (let i = 0; i < 60 && dictionary.perfVersion === version; i++) await frame();
  return dictionary.perfVersion !== version ? samples[samples.length - 1] : undefined;
}

export async function runBench(dictionary: Dictionary): Promise<boolean> {
  if (!isTauri()) return false;
  const plan = await invoke<Plan | null>('bench_plan');
  if (!plan) return false;
  const log = (message: string) => invoke('bench_log', { message });
  await log(`start: ${plan.queries.length} queries, ${plan.entries.length} entries, hidden=${document.hidden}`);
  const suggest: number[] = [];
  const entry: number[] = [];
  for (const [index, query] of plan.queries.entries()) {
    if (index % 25 === 0) await log(`query ${index}: ${suggest.length} samples`);
    for (let length = 1; length <= query.length; length++) {
      const version = dictionary.perfVersion;
      await dictionary.lookup(query.slice(0, length));
      const sample = await nextSample(dictionary, dictionary.samples.suggest, version);
      if (sample !== undefined) suggest.push(sample);
    }
  }
  await log(`entries: ${suggest.length} suggest samples`);
  for (const key of plan.entries) {
    const version = dictionary.perfVersion;
    await dictionary.open(key);
    const sample = await nextSample(dictionary, dictionary.samples.entry, version);
    if (sample !== undefined) entry.push(sample);
  }
  await invoke('bench_report', {
    report: { suggest: summarize(suggest), entry: summarize(entry), samples: { suggest, entry } },
  });
  return true;
}
