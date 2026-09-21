const SPRITE_W = 192;
const SPRITE_H = 208;
const FRAME_COUNT = 6;
const FRAME_DURATIONS = {
  idle:     [0.28, 0.11, 0.11, 0.14, 0.14, 0.32],
  sneezing: [0.18, 0.15, 0.12, 0.14, 0.17, 0.25],
  kneading: [0.18, 0.18, 0.18, 0.18, 0.18, 0.24],
  rubnose:   [0.14, 0.15, 0.18, 0.18, 0.16, 0.20],
  contented: [0.16, 0.16, 0.18, 0.20, 0.24, 0.45],
  headtilt:  [0.15, 0.18, 0.20, 0.20, 0.18, 0.22],
  yawning:   [0.18, 0.20, 0.24, 0.32, 0.22, 0.28],
  stretching:[0.18, 0.20, 0.26, 0.28, 0.20, 0.22],
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
    this.queue = [];
    this.urls = {
      idle: assetUrl('idle'),
      sneezing: assetUrl('sneezing'),
      kneading: assetUrl('kneading'),
      rubnose: assetUrl('rubnose'),
      contented: assetUrl('contented'),
      headtilt: assetUrl('headtilt'),
      yawning: assetUrl('yawning'),
      stretching: assetUrl('stretching'),
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

  playSequence(actions) {
    const valid = (actions || []).filter((action) => action !== 'idle' && FRAME_DURATIONS[action]);
    if (!valid.length) return;
    this.queue = valid.slice(1);
    this.setAction(valid[0]);
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
      } else if (this.queue.length) {
        this.setAction(this.queue.shift());
        return;
      } else {
        this.setAction('idle');
        return;
      }
    }
    this.render();
    this.scheduleNext();
  }
}
