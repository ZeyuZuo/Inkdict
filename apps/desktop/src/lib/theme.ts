export function installTheme(css: string): void {
  let style = document.getElementById('zdict-entry-theme') as HTMLStyleElement | null;
  if (!style) {
    style = document.createElement('style');
    style.id = 'zdict-entry-theme';
    document.head.append(style);
  }
  style.textContent = css;
  warmFonts(css);
}

/** Lay out a hidden line in every theme font (Latin, IPA, CJK) right away,
 *  so the first entry does not pay for font loading (it took ~300 ms). */
function warmFonts(css: string): void {
  document.getElementById('zdict-font-warmup')?.remove();
  const tokens = [...new Set([...css.matchAll(/--font-([\w-]+):/g)].map(match => match[1]))];
  const box = document.createElement('div');
  box.id = 'zdict-font-warmup';
  box.setAttribute('aria-hidden', 'true');
  box.style.cssText = 'position:absolute;left:-9999px;top:0;visibility:hidden;pointer-events:none';
  for (const token of tokens) {
    for (const weight of [400, 700]) {
      const line = document.createElement('span');
      line.style.cssText = `font-family:var(--font-${token});font-weight:${weight}`;
      line.textContent = 'Aa ˈæpl 苹果 Ⅻ';
      box.append(line);
    }
  }
  document.body.append(box);
  void box.offsetHeight;
}
