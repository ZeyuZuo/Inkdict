import type { Settings, Word } from './types';

// All definitions and examples below are invented fixtures, not dictionary text.
export const words = [
  { key: 'moonjar', word: 'moonjar', pron: 'ˈmuːndʒɑː', zh: '盛放夜间小念头的想象容器',
    definition: 'An imaginary jar for small thoughts collected at night.', example: 'I left a tiny wish in my moonjar.', family: 'moonjarring' },
  { key: 'hushleaf', word: 'hushleaf', pron: 'ˈhʌʃliːf', zh: '夹在书页间提醒人休息的叶子',
    definition: 'A leaf tucked into a book as a reminder to pause.', example: 'A hushleaf marked the page where we rested.', family: 'hushleafed' },
  { key: 'fernlight', word: 'fernlight', pron: 'ˈfɜːnlaɪt', zh: '透过蕨叶的柔和光线',
    definition: 'Soft light passing through a patch of ferns.', example: 'We drew little maps in the fernlight.', family: 'fernlit' },
  { key: 'cloudstep', word: 'cloudstep', pron: 'ˈklaʊdstep', zh: '为了看云而停顿的一小步',
    definition: 'A small pause in a walk to look at a cloud.', example: 'Our walk included a cloudstep by the gate.', family: 'cloudstepping' },
  { key: 'pebblepost', word: 'pebblepost', pron: 'ˈpeblpəʊst', zh: '用小石头压住的友好便条',
    definition: 'A friendly note held in place by a little stone.', example: 'A pebblepost waited beside the blue door.', family: 'pebbleposting' },
  { key: 'sunthread', word: 'sunthread', pron: 'ˈsʌnθred', zh: '穿过窗帘的一线阳光',
    definition: 'A thin line of sunshine slipping through a curtain.', example: 'One sunthread crossed the quiet desk.', family: 'sunthreaded' },
];

/** Word list rows of the invented entries: each headword, and its phrase at anchor 2. */
export function rows(): Word[] {
  return words.flatMap(w => [
    { form: w.word, kind: 'headword', anchor: 0, summary: `n. ${w.zh}` },
    { form: `${w.word} moment`, kind: 'phrase', anchor: 2, summary: '留意微小事物的片刻' },
  ].map(row => ({
    group: 'prefix', entry: w.key, headword: w.word, homograph: null, readings: [], summary_lang: 'zh-Hans',
    distance: null, rank: row.anchor, target: null, ...row,
  }) as Word));
}

export function escape(text: string): string {
  return text.replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]!);
}
function link(text: string, target: Record<string, string>): string {
  return `<a href="#" class="dx-link" data-link='${escape(JSON.stringify({ ...target, text }))}'>${escape(text)}</a>`;
}
function sound(): string {
  // A short locally generated tone exercises playback without a network resource.
  const bytes = new Uint8Array(44 + 1600);
  const view = new DataView(bytes.buffer);
  for (const [offset, text] of [[0, 'RIFF'], [8, 'WAVE'], [12, 'fmt '], [36, 'data']] as const) {
    [...text].forEach((c, i) => { bytes[offset + i] = c.charCodeAt(0); });
  }
  view.setUint32(4, bytes.length - 8, true); view.setUint32(16, 16, true);
  view.setUint16(20, 1, true); view.setUint16(22, 1, true);
  view.setUint32(24, 8000, true); view.setUint32(28, 8000, true);
  view.setUint16(32, 1, true); view.setUint16(34, 8, true); view.setUint32(40, 1600, true);
  for (let i = 0; i < 1600; i++) bytes[44 + i] = 128 + Math.round(16 * Math.sin(i * 0.35) * (1 - i / 1600));
  return `data:audio/wav;base64,${btoa(String.fromCharCode(...bytes))}`;
}
export function renderMock(key: string, settings: Settings): string | null {
  const w = words.find(w => w.key === key);
  if (!w) return null;
  const next = words[(words.indexOf(w) + 1) % words.length];
  const image = `data:image/svg+xml,${encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="160" height="80"><rect width="160" height="80" rx="12" fill="#e9eee4"/><circle cx="80" cy="40" r="21" fill="#a4b995"/></svg>')}`;
  const example = `<div class="dx-example" style="margin-top:.7em;font-style:italic">${w.example}</div>`;
  return `<article class="dx-entry" data-anchor="0" lang="en">
    <div class="dx-head" style="margin-bottom:1.8em">
      <div class="dx-headword" style="font-size:2.4em;font-weight:700;line-height:1.2">${w.word}</div>
      <div style="margin-top:.6em;color:var(--c-muted)"><span style="font-style:italic">noun · 创作词</span>
      ${settings.hide_pronunciation ? '' : `<span class="dx-pronunciation" style="margin-left:1em">/${w.pron}/ <button class="dx-icon" aria-label="播放示例声音" title="播放示例声音" data-audio="data:audio/wav;base64,AA== ${sound()}">◖))</button></span>`}</div>
    </div>
    <section class="dx-sense" data-anchor="1" style="position:relative;padding-left:1.6em;margin-bottom:1.8em">
      <span class="dx-marker" style="width:1.6em;color:var(--c-muted)">1</span>
      <div>${w.definition}</div>
      <div lang="zh-CN" style="color:var(--c-muted);margin-top:.3em">${w.zh}。</div>
      ${settings.hide_examples ? '' : example}
    </section>
    <section data-anchor="2" class="dx-phrase" style="border-top:1px solid var(--c-rule);padding-top:1.2em;margin-bottom:1.5em">
      <div class="dx-title">短语</div><strong>${w.word} moment</strong>
      <div>A quiet moment to notice something small.</div>
      <div lang="zh-CN" style="color:var(--c-muted)">留意微小事物的片刻。</div>
    </section>
    <aside class="dx-word_family" style="padding:1em 1.2em;background:var(--c-box);border-radius:.5em">
      <div class="dx-title">词族</div><strong>${w.family}</strong> <span style="color:var(--c-muted)">衍生形式 · 示例</span>
    </aside>
    <div style="margin-top:1.5em">相关词 ${link(next.word, { entry: next.key })}
      · ${link('查看词族', { entry: w.key, anchor_type: 'wfg' })}
      · ${link('跳到短语', { entry: w.key, anchor: 'moment' })}
      · ${link('fernlight', {})}</div>
    ${w.key === 'moonjar' ? `<figure style="margin:1.5em 0 0"><img width="160" height="80" alt="想象月亮的简笔图" src="data:image/png;base64,AA==" data-fallback="${image}"/><figcaption style="color:var(--c-muted);font-size:.8em">示意图 · 本地生成</figcaption></figure>` : ''}
  </article>`;
}
