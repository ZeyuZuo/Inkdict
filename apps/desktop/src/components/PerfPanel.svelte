<script lang="ts">
  import { summarize, type Samples } from '../lib/perf';
  let { samples, version, onclose }: { samples: Samples; version: number; onclose: () => void } = $props();
  const metrics = $derived.by(() => {
    void version;
    return [{ label: '联想', ...summarize(samples.suggest) }, { label: '词条', ...summarize(samples.entry) }];
  });
</script>

<aside class="perf-panel" aria-label="延迟统计">
  <div class="panel-title"><h2>延迟统计 <span>毫秒 · 最近 200 次</span></h2><button aria-label="关闭延迟统计" onclick={onclose}>×</button></div>
  <table><thead><tr><th>操作</th><th>次数</th><th>p50</th><th>p95</th><th>最大</th></tr></thead>
    <tbody>{#each metrics as metric}<tr><th>{metric.label}</th><td>{metric.count}</td><td>{metric.p50.toFixed(1)}</td><td>{metric.p95.toFixed(1)}</td><td>{metric.max.toFixed(1)}</td></tr>{/each}</tbody>
  </table>
  <p>从发起操作到画面更新（包含渲染等待）。</p>
</aside>
