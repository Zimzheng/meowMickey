const SPRITE_W = 192;
const SPRITE_H = 208;
const FRAME_COUNT = 6;
const FRAME_DURATIONS = {
  idle:     [0.28, 0.11, 0.11, 0.14, 0.14, 0.32],
  sneezing: [0.18, 0.15, 0.12, 0.14, 0.17, 0.25],
  kneading: [0.18, 0.18, 0.18, 0.18, 0.18, 0.24],
};

function assetUrl(action) {
  return `sprites/${action}.png`;
}

export class SpriteSheet {
  constructor(element, scale = 1) {
    this.element = element;
    this.action = 'idle';
    this.frameIndex = 0;
    this.timerId = null;
    this.scale = scale;
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

  setScale(scale) {
    this.scale = scale;
    this.render();
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
    // At scale > 1 the div is larger than the source sprite. background-size: 100% 100%
    // (in CSS) scales the image to fill the div. background-position operates in
    // element coordinates, so we offset by frameIndex * (sprite_width * scale) pixels
    // to land on the right source frame.
    const offset = -this.frameIndex * SPRITE_W * this.scale;
    this.element.style.backgroundPosition = `${offset}px 0`;
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
