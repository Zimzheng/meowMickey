const SPRITE_W = 192;
const SPRITE_H = 208;
const FRAME_COUNT = 6;
const FRAME_DURATIONS = {
  idle:     [0.28, 0.11, 0.11, 0.14, 0.14, 0.32],
  sneezing: [0.18, 0.15, 0.12, 0.14, 0.17, 0.25],
  kneading: [0.18, 0.18, 0.18, 0.18, 0.18, 0.24],
};

const tauri = window.__TAURI__;
const convertFileSrc = tauri?.core?.convertFileSrc || tauri?.convertFileSrc;

function assetUrl(action) {
  if (convertFileSrc) return convertFileSrc(`${action}.png`);
  return `${action}.png`;
}

export class SpriteSheet {
  constructor(element) {
    this.element = element;
    this.action = 'idle';
    this.frameIndex = 0;
    this.timerId = null;
    this.urls = {
      idle: assetUrl('idle'),
      sneezing: assetUrl('sneezing'),
      kneading: assetUrl('kneading'),
    };
    this.preload();
    this.setAction('idle');
  }

  preload() {
    for (const url of Object.values(this.urls)) {
      const img = new Image();
      img.src = url;
    }
  }

  setAction(action) {
    if (!FRAME_DURATIONS[action]) return;
    if (this.timerId !== null) {
      clearTimeout(this.timerId);
      this.timerId = null;
    }
    this.action = action;
    this.frameIndex = 0;
    this.element.style.backgroundImage = `url("${this.urls[action]}")`;
    this.render();
    this.scheduleNext();
  }

  render() {
    this.element.style.backgroundPosition = `-${this.frameIndex * SPRITE_W}px 0`;
  }

  scheduleNext() {
    const durations = FRAME_DURATIONS[this.action];
    const dur = durations[this.frameIndex] ?? 0.2;
    this.timerId = setTimeout(() => this.advance(), dur * 1000);
  }

  advance() {
    this.timerId = null;
    this.frameIndex += 1;
    if (this.frameIndex >= FRAME_COUNT) {
      if (this.action === 'idle') {
        this.frameIndex = 0;
      } else {
        this.setAction('idle');
        return;
      }
    }
    this.render();
    this.scheduleNext();
  }
}
