export function installTheme(css: string): void {
  let style = document.getElementById('zdict-entry-theme') as HTMLStyleElement | null;
  if (!style) {
    style = document.createElement('style');
    style.id = 'zdict-entry-theme';
    document.head.append(style);
  }
  style.textContent = css;
}
