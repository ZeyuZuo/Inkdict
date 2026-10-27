export type Metric = 'words' | 'entry';
export interface Samples { words: number[]; entry: number[] }
export const createSamples = (): Samples => ({ words: [], entry: [] });
export function record(samples: number[], milliseconds: number): void {
  if (!Number.isFinite(milliseconds) || milliseconds < 0) return;
  samples.push(milliseconds);
  if (samples.length > 200) samples.splice(0, samples.length - 200);
}
// Nearest-rank percentiles: a recorded observation, including for small samples.
export function percentile(samples: readonly number[], fraction: number): number {
  if (!samples.length) return 0;
  const sorted = [...samples].sort((a, b) => a - b);
  const rank = Math.ceil(Math.min(1, Math.max(0, fraction)) * sorted.length);
  return sorted[Math.max(0, rank - 1)];
}
export function summarize(samples: readonly number[]) {
  return { count: samples.length, p50: percentile(samples, 0.5), p95: percentile(samples, 0.95), max: Math.max(0, ...samples) };
}

declare global { interface Window { __zdictPerf: Samples } }
