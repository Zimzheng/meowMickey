const tauri = window.__TAURI__;
const invoke = tauri?.core?.invoke || tauri?.invoke;
const DRAG_THRESHOLD = 4;
const CLICK_DELAY_MS = 300;

export function installMouseHandling(element, { onSingleClick, onDoubleClick, onRapidClick, onRightClick }) {
  let press = null;
  let pendingClick = null;
  let clickCount = 0;

  const cancelClicks = () => {
    clearTimeout(pendingClick);
    pendingClick = null;
    clickCount = 0;
  };
  const finishDrag = () => {
    press = null;
    invoke?.('end_drag').catch(() => {});
  };

  element.addEventListener('pointerdown', (event) => {
    if (event.button !== 0 || event.isPrimary === false) return;
    clearTimeout(pendingClick);
    pendingClick = null;
    press = { x: event.screenX, y: event.screenY, id: event.pointerId, dragging: false };
    element.setPointerCapture?.(event.pointerId);
  });

  element.addEventListener('pointermove', (event) => {
    if (!press || press.id !== event.pointerId || press.dragging) return;
    // Starting a native drag on pointerdown consumes mouseup in Windows WebView2.
    // Screen coordinates remain stable when the window itself moves.
    if (Math.hypot(event.screenX - press.x, event.screenY - press.y) <= DRAG_THRESHOLD) return;
    press.dragging = true;
    cancelClicks();
    if (element.hasPointerCapture?.(event.pointerId)) element.releasePointerCapture(event.pointerId);
    if (invoke) invoke('native_drag').catch(() => {}).finally(finishDrag);
  });

  element.addEventListener('pointerup', (event) => {
    if (event.button !== 0 || !press || press.id !== event.pointerId) return;
    const dragging = press.dragging;
    press = null;
    if (element.hasPointerCapture?.(event.pointerId)) element.releasePointerCapture(event.pointerId);
    if (dragging) { finishDrag(); return; }
    // Count completed clicks ourselves: native WebViews differ in event.detail.
    clearTimeout(pendingClick);
    clickCount += 1;
    pendingClick = setTimeout(() => {
      const count = clickCount;
      pendingClick = null;
      clickCount = 0;
      if (count >= 3) onRapidClick?.();
      else if (count === 2) onDoubleClick?.();
      else onSingleClick?.();
    }, CLICK_DELAY_MS);
  });

  element.addEventListener('pointercancel', () => { cancelClicks(); finishDrag(); });
  element.addEventListener('lostpointercapture', () => {
    if (press && !press.dragging) { cancelClicks(); press = null; }
  });
  window.addEventListener('blur', () => { cancelClicks(); press = null; });
  element.addEventListener('contextmenu', (event) => {
    event.preventDefault();
    cancelClicks();
    onRightClick?.();
  });
}
