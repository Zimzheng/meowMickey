import { layoutDiaryDetails } from './diary-layout.js';
import { SpriteSheet } from './sprite-v121.js';
import { installMouseHandling } from './mouse.js';

const tauri = window.__TAURI__;
const invoke = tauri?.core?.invoke || tauri?.invoke;
const listen = tauri?.event?.listen || tauri?.listen;

const isSidePanel = new URLSearchParams(location.search).get('side') === 'hydration';
if (isSidePanel) document.body.classList.add('side-panel');
const spriteElement = document.getElementById('sprite');
let sprite = new SpriteSheet(spriteElement);
const hydrationPanel = document.getElementById('hydration');
const hydrationMessage = document.getElementById('hydration-message');
const hydrationToday = document.getElementById('hydration-today');
const hydrationAmount = document.getElementById('hydration-amount');
const toast = document.getElementById('toast');
const toastMessage = document.getElementById('toast-message');
const toastUndo = document.getElementById('toast-undo');
const hydrationUndo = document.getElementById('hydration-undo');
let hydrationSource = 'reminder';
let toastTimer = null;
let hydrationSuccessTimer = null;

let rules = {
  sneezeEveryMinutes: 30,
  kneadEveryMinutes: 5,
  singleClick: 'kneading',
  doubleClick: 'sneezing',
  behaviorEnabled: true,
};

const hydrationMessages = [
  '该喝水啦，米奇也口渴了～',
  '和米奇一起喝水吧！',
  '休息一下，补充一点水分吧。',
  '米奇在等你一起咕噜噜～',
];

function localDateParts(date = new Date()) {
  const pad = (n) => String(n).padStart(2, '0');
  return { date: `${date.getFullYear()}-${pad(date.getMonth()+1)}-${pad(date.getDate())}`, time: `${pad(date.getHours())}:${pad(date.getMinutes())}` };
}

async function refreshHydrationSummary() {
  if (!invoke) return;
  const { date } = localDateParts();
  try {
    const summary = await invoke('get_hydration_summary', { localDate: date });
    hydrationToday.textContent = `今天已喝 ${summary.totalMl} ml · ${summary.recordCount} 次`;
  } catch (e) { console.warn('hydration summary failed', e); }
}

async function showHydrationPrompt(source = 'reminder') {
  if (!isSidePanel) {
    if (invoke) await invoke('set_hydration_panel_open', { open: true });
    return;
  }
  clearTimeout(hydrationSuccessTimer);
  hydrationPanel.classList.remove('success');
  hydrationUndo.hidden = true;
  hydrationSource = source;
  hydrationMessage.textContent = hydrationMessages[Math.floor(Math.random() * hydrationMessages.length)];
  hydrationPanel.hidden = false;
  diaryShare.hidden = false;
  refreshHydrationSummary();
}

async function hideHydrationPrompt() {
  clearTimeout(hydrationSuccessTimer);
  hydrationPanel.hidden = true;
  if (invoke) await invoke('set_hydration_panel_open', { open: false }).catch(() => {});
}

function showToast(message, canUndo = false) {
  toastMessage.textContent = message;
  toastUndo.hidden = !canUndo;
  toast.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { toast.hidden = true; }, 3200);
}

async function recordWater(amount) {
  const amountMl = Number(amount);
  if (!Number.isFinite(amountMl) || amountMl < 10 || amountMl > 3000) {
    showToast('请输入 10–3000 ml'); return;
  }
  const now = new Date();
  const local = localDateParts(now);
  try {
    const summary = await invoke('record_hydration', { amountMl: Math.round(amountMl), recordedAtMs: now.getTime(), localDate: local.date, localTime: local.time, source: hydrationSource });
    hydrationAmount.value = '';
    logInteraction('hydration_record', { amountMl: Math.round(amountMl), result: 'accepted' });
    hydrationPanel.classList.add('success');
    hydrationMessage.textContent = `咕噜噜～和米奇一起喝了 ${Math.round(amountMl)} ml！`;
    hydrationToday.textContent = `今天共 ${summary.totalMl} ml · ${summary.recordCount} 次`;
    hydrationUndo.hidden = false;
    // Keep the acknowledgement visible through the full drinking animation
    // and the following contented pose.
    hydrationSuccessTimer = setTimeout(() => hideHydrationPrompt(), 6200);
  } catch (e) { showToast(String(e)); }
}

document.querySelectorAll('[data-ml]').forEach((button) => button.addEventListener('click', () => recordWater(button.dataset.ml)));
document.getElementById('hydration-form').addEventListener('submit', (event) => { event.preventDefault(); recordWater(hydrationAmount.value); });
document.getElementById('hydration-close').addEventListener('click', () => hideHydrationPrompt());
document.getElementById('hydration-snooze').addEventListener('click', async () => {
  if (invoke) await invoke('snooze_hydration', { minutes: 10 }).catch(() => {});
  await hideHydrationPrompt(); showToast('好呀，10 分钟后米奇再来～');
});
async function undoLatestHydration() {
  const { date } = localDateParts();
  try {
    const summary = await invoke('undo_hydration', { localDate: date });
    hydrationMessage.textContent = '已撤销本次记录～';
    hydrationToday.textContent = `今天共 ${summary.totalMl} ml · ${summary.recordCount} 次`;
    hydrationUndo.hidden = true;
    hydrationSuccessTimer = setTimeout(() => hideHydrationPrompt(), 1800);
  } catch (e) { showToast(String(e)); }
}
toastUndo.addEventListener('click', undoLatestHydration);
hydrationUndo.addEventListener('click', undoLatestHydration);

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
  onSingleClick: () => { logInteraction('pet_click', { action: rules.singleClick }); triggerAction(rules.singleClick); },
  onDoubleClick: () => { logInteraction('pet_double_click', { action: rules.doubleClick }); triggerAction(rules.doubleClick); },
  onRapidClick: () => { logInteraction('pet_rapid_click'); triggerAction('headtilt'); },
  onRightClick: () => {
    if (invoke) invoke('show_context_menu').catch(() => {});
  },
});

(async function init() {
  await loadInitialScale();
  await loadInitialRules();
  if (isSidePanel) await showHydrationPrompt('manual');
  if (invoke) {
    const reportHour = () => invoke('set_local_hour', { hour: new Date().getHours() }).catch(() => {});
    reportHour();
    setInterval(reportHour, 5 * 60 * 1000);
  }

  if (listen) {
    await listen('trigger', (event) => {
      const actions = event.payload?.actions;
      if (Array.isArray(actions)) {
        sprite.playSequence(actions);
        if (actions.includes('thirsty')) showHydrationPrompt('reminder');
      }
    });
    if (isSidePanel) {
      await listen('hydration_side_open', () => {
        document.getElementById('share-preview').hidden = true;
        showHydrationPrompt('manual');
      });
    }
    await listen('hydration_prompt', async () => { await showHydrationPrompt('manual'); sprite.playSequence(['thirsty']); });
    await listen('hydration_undo_request', async () => {
      const { date } = localDateParts();
      try {
        const summary = await invoke('undo_hydration', { localDate: date });
        await showHydrationPrompt('manual');
        hydrationPanel.classList.add('success');
        hydrationMessage.textContent = '已撤销最近一次喝水记录～';
        hydrationToday.textContent = `今天共 ${summary.totalMl} ml · ${summary.recordCount} 次`;
        hydrationUndo.hidden = true;
        hydrationSuccessTimer = setTimeout(() => hideHydrationPrompt(), 2400);
      } catch (e) { showToast(String(e)); }
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

// Companion dialogue and local interaction journal.
const companionBubble = document.getElementById('companion-bubble');
const companionText = document.getElementById('companion-text');
const companionPrimary = document.getElementById('companion-primary');
const companionSnooze = document.getElementById('companion-snooze');
const companionClose = document.getElementById('companion-close');
const diaryShare = document.getElementById('diary-share');
let companionTimer = null;
let companionKind = 'missing_you';
const companionMessages = [
  '米奇想你了，摸摸米奇吧～',
  '你忙了好久，米奇一直在这里陪你。',
  '米奇探头看你好久啦，要不要和米奇玩一下？',
];
function journal() { try { return JSON.parse(localStorage.getItem('mickey-interactions') || '[]'); } catch { return []; } }
function logInteraction(type, metadata = {}) {
  const items = journal(); items.push({ type, createdAt: new Date().toISOString(), ...metadata });
  localStorage.setItem('mickey-interactions', JSON.stringify(items.slice(-500)));
  localStorage.setItem('mickey-last-interaction', String(Date.now()));
}
function showCompanion(kind = 'missing_you', message) {
  companionKind = kind;
  companionText.textContent = message || (kind === 'hydration' ? hydrationMessages[Math.floor(Math.random() * hydrationMessages.length)] : companionMessages[Math.floor(Math.random() * companionMessages.length)]);
  companionPrimary.textContent = kind === 'hydration' ? '一起喝水' : '摸摸米奇';
  companionBubble.hidden = false;
  clearTimeout(companionTimer); companionTimer = setTimeout(() => { companionBubble.hidden = true; }, 9000);
  logInteraction(kind === 'hydration' ? 'hydration_reminder' : 'missing_you', { result: 'shown', message: companionText.textContent });
}
function hideCompanion() { companionBubble.hidden = true; clearTimeout(companionTimer); }
companionClose?.addEventListener('click', hideCompanion);
companionSnooze?.addEventListener('click', () => { hideCompanion(); localStorage.setItem('mickey-companion-snooze', String(Date.now() + 30 * 60 * 1000)); showToast('好呀，米奇半小时后再来～'); });
companionPrimary?.addEventListener('click', async () => {
  logInteraction(companionKind === 'hydration' ? 'hydration_reminder' : 'pet_click', { result: 'accepted' });
  if (companionKind === 'hydration') await showHydrationPrompt('reminder');
  else triggerAction('headtilt');
  hideCompanion();
});
function maybeCompanionPrompt() {
  if (document.visibilityState === 'hidden') return;
  const now = Date.now(); const last = Number(localStorage.getItem('mickey-last-interaction') || now);
  const snooze = Number(localStorage.getItem('mickey-companion-snooze') || 0);
  if (now < snooze || companionBubble.hidden === false || hydrationPanel.hidden === false) return;
  const today = new Date().toISOString().slice(0, 10);
  const shownToday = journal().filter(x => x.createdAt?.startsWith(today) && (x.type === 'missing_you' || x.type === 'hydration_reminder')).length;
  if (shownToday >= 6) return;
  if (now - last >= 90 * 60 * 1000) showCompanion('missing_you');
}
setInterval(maybeCompanionPrompt, 60 * 1000);

async function shareDiary() {
  clearTimeout(hydrationSuccessTimer);
  clearTimeout(toastTimer);
  hideCompanion();
  hydrationPanel.hidden = true;
  toast.hidden = true;
  const { date } = localDateParts();
  let summary = { totalMl: 0, recordCount: 0 };
  if (invoke) summary = await invoke('get_hydration_summary', { localDate: date }).catch(() => summary);
  const events = journal().filter(x => x.createdAt?.startsWith(date));
  const labels = { hydration_reminder: '喝水提醒', hydration_record: '喝水记录', missing_you: '米奇想念', pet_click: '摸摸米奇', pet_double_click: '双击互动', pet_rapid_click: '快速点击', share_diary: '分享日记' };
  const counts = events.reduce((map, event) => { const label = labels[event.type] || '其他互动'; map[label] = (map[label] || 0) + 1; return map; }, {});
  const detail = Object.entries(counts).map(([label, count]) => `${label} ${count}次`).join(' · ') || '暂无互动';
  const canvas = document.createElement('canvas'); canvas.width = 1080;
  const ctx = canvas.getContext('2d');
  ctx.font = '26px sans-serif';
  const layout = layoutDiaryDetails(ctx, detail);
  // Assigning canvas height resets the drawing state, so set styles below.
  canvas.height = layout.height;
  ctx.fillStyle = '#eef8fb'; ctx.fillRect(0, 0, canvas.width, canvas.height);
  ctx.fillStyle = '#fffdf8'; ctx.roundRect(70, 70, 940, canvas.height - 140, 36); ctx.fill();
  ctx.fillStyle = '#4d91ae'; ctx.font = 'bold 54px sans-serif'; ctx.fillText('米奇陪伴日记', 130, 180);
  ctx.fillStyle = '#77909e'; ctx.font = '28px sans-serif'; ctx.fillText(date, 130, 240);
  ctx.fillStyle = '#385462'; ctx.font = 'bold 38px sans-serif'; ctx.fillText(`今日喝水：${summary.totalMl} ml`, 130, 390); ctx.fillText(`互动总计：${events.length} 次`, 130, 480);
  ctx.font = '26px sans-serif';
  layout.lines.forEach((line, index) => ctx.fillText(line, 130, layout.detailY + index * layout.lineHeight));
  ctx.font = '30px sans-serif'; ctx.fillText('米奇今天也一直陪着你～', 130, layout.closingY);
  const cat = new Image(); cat.src = 'sprites/idle.png'; await new Promise(resolve => { cat.onload = resolve; cat.onerror = resolve; }); if (cat.complete && cat.naturalWidth) ctx.drawImage(cat, 0, 0, 192, 208, 690, layout.catY, 300, 325);
  const imageUrl = canvas.toDataURL('image/png');
  document.getElementById('share-preview-image').src = imageUrl;
  if (invoke && !isSidePanel) await invoke('set_hydration_panel_open', { open: true });
  document.getElementById('share-preview').hidden = false;
  window.__mickeyShare = { imageUrl, text: `🐱 米奇陪伴日记｜${date}\n💧 今日喝水：${summary.totalMl} ml\n✨ 互动总计：${events.length} 次\n${detail}\n米奇今天也一直陪着你～` };
  logInteraction('share_diary', { format: 'preview' });
}
diaryShare?.addEventListener('click', shareDiary);
async function closeSharePreview() {
  clearTimeout(hydrationSuccessTimer);
  clearTimeout(toastTimer);
  document.getElementById('share-preview').hidden = true;
  document.getElementById('share-preview-image').removeAttribute('src');
  hydrationPanel.hidden = true;
  toast.hidden = true;
  hideCompanion();
  // Commit the transparent frame before shrinking the native window.
  await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  if (invoke) await invoke('set_hydration_panel_open', { open: false });
}
document.getElementById('share-preview-close')?.addEventListener('click', closeSharePreview);
document.getElementById('share-preview')?.addEventListener('click', (event) => { if (event.target.id === 'share-preview') closeSharePreview(); });
document.getElementById('share-copy-text')?.addEventListener('click', async () => { try { await navigator.clipboard.writeText(window.__mickeyShare?.text || ''); } catch {} });
document.getElementById('share-copy-image')?.addEventListener('click', async () => {
  if (!invoke || !window.__mickeyShare?.imageUrl) return;
  await invoke('copy_image_to_clipboard', { dataUrl: window.__mickeyShare.imageUrl }).catch(() => {});
});
const shareImage = document.getElementById('share-preview-image');
shareImage?.addEventListener('contextmenu', event => event.preventDefault());
