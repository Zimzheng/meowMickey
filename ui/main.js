import { SpriteSheet } from './sprite.js';
import { installMouseHandling } from './mouse.js';

const spriteElement = document.getElementById('sprite');
const sprite = new SpriteSheet(spriteElement);

installMouseHandling(spriteElement, {
  onSingleClick: () => sprite.setAction('kneading'),
  onDoubleClick: () => sprite.setAction('sneezing'),
  onRightClick: () => {
    const tauri = window.__TAURI__;
    const invoke = tauri?.core?.invoke || tauri?.invoke;
    if (invoke) invoke('show_context_menu').catch(() => {});
  },
});

window.__mickey = { sprite };