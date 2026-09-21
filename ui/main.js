import { SpriteSheet } from './sprite.js';
import { installMouseHandling } from './mouse.js';

const tauri = window.__TAURI__;
const invoke = tauri?.core?.invoke || tauri?.invoke;
const listen = tauri?.event?.listen || tauri?.listen;

const spriteElement = document.getElementById('sprite');
let sprite = new SpriteSheet(spriteElement);

let rules = {
  sneezeEveryMinutes: 30,
  kneadEveryMinutes: 5,
  singleClick: 'kneading',
  doubleClick: 'sneezing',
  behaviorEnabled: true,
};

function applyScale(scale) {
  const w = 192 * scale;
  const h = 208 * scale;
  document.documentElement.style.setProperty('--sprite-w', `${w}px`);
  document.documentElement.style.setProperty('--sprite-h', `${h}px`);
  sprite.setScale(scale);
}

async function loadInitialRules() {
  if (!invoke) return;
  try {
    rules = await invoke('get_rules');
  } catch (e) {
    console.warn('get_rules failed; using defaults', e);
  }
}

async function loadInitialScale() {
  if (!invoke) return;
  try {
    const scale = await invoke('get_scale');
    applyScale(scale);
  } catch (e) {
    console.warn('get_scale failed; using default 1.0', e);
  }
}

function triggerAction(action) {
  if (!action || action === 'idle') return;
  if (invoke) {
    invoke('trigger_action', { action }).catch((e) => {
      console.warn('trigger_action failed', e);
    });
  } else {
    // Browser-only preview has no backend event bridge.
    sprite.setAction(action);
  }
}

installMouseHandling(spriteElement, {
  onSingleClick: () => triggerAction(rules.singleClick),
  onDoubleClick: () => triggerAction(rules.doubleClick),
  onRapidClick: () => triggerAction('headtilt'),
  onRightClick: () => {
    if (invoke) invoke('show_context_menu').catch(() => {});
  },
});

(async function init() {
  await loadInitialScale();
  await loadInitialRules();
  if (invoke) {
    const reportHour = () => invoke('set_local_hour', { hour: new Date().getHours() }).catch(() => {});
    reportHour();
    setInterval(reportHour, 5 * 60 * 1000);
  }

  if (listen) {
    await listen('trigger', (event) => {
      const actions = event.payload?.actions;
      if (Array.isArray(actions)) sprite.playSequence(actions);
    });
    await listen('rules_changed', (event) => {
      if (event.payload) rules = event.payload;
    });
    await listen('scale_changed', (event) => {
      if (typeof event.payload === 'number') applyScale(event.payload);
    });
    await listen('paused', (event) => {
      console.log('paused:', event.payload);
    });
  }
})();

window.__mickey = { sprite, get rules() { return rules; } };
