export class AudioPlayer {
  private audio: HTMLAudioElement | null = null;
  private sequence = 0;
  constructor(private failed: (message: string) => void) {}
  play(urls: string[]) {
    this.stop();
    const sequence = this.sequence;
    const attempt = (index: number) => {
      if (sequence !== this.sequence) return;
      if (index >= urls.length) { this.failed('暂时无法播放发音。'); return; }
      const audio = new Audio(urls[index]);
      this.audio = audio;
      let advanced = false;
      const fallback = () => {
        if (advanced || sequence !== this.sequence) return;
        advanced = true;
        audio.onerror = null;
        audio.pause();
        attempt(index + 1);
      };
      audio.onerror = fallback;
      void audio.play().catch(fallback);
    };
    attempt(0);
  }
  stop() {
    this.sequence++;
    if (this.audio) { this.audio.onerror = null; this.audio.pause(); this.audio = null; }
  }
}

export function fallbackImage(event: Event): void {
  const image = event.target;
  if (!(image instanceof HTMLImageElement) || !image.hasAttribute('data-fallback')) return;
  const urls = (image.dataset.fallback ?? '').split(/\s+/).filter(Boolean);
  if (urls.length) {
    image.dataset.fallback = urls.slice(1).join(' ');
    image.src = urls[0];
  } else {
    image.removeAttribute('data-fallback');
    image.style.visibility = 'hidden';
  }
}
