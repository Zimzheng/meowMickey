const tauri = window.__TAURI__;
const invoke = tauri?.core?.invoke || tauri?.invoke;
const DRAG_THRESHOLD = 3;
const CLICK_DELAY_MS = 250;

export function installMouseHandling(element, { onSingleClick, onDoubleClick, onRapidClick, onRightClick }) {
  let dragStartClient = null;
  let moved = false;
  let pendingSingleClick = null;

  const dpr = window.devicePixelRatio || 1;

  element.addEventListener('mousedown', (event) => {
    if (event.button !== 0) return;
    dragStartClient = { x: event.clientX, y: event.clientY };
    moved = false;
    if (invoke) {
      // Hand the drag off to AppKit — it handles all subsequent window movement
      // directly, no IPC roundtrip per mousemove.
      invoke('native_drag').catch(() => {});
    }
  });

  element.addEventListener('mousemove', (event) => {
    if (!dragStartClient) return;
    // AppKit is already moving the window. We only track distance locally
    // to disambiguate click vs drag — no IPC, no set_position here.
    const dx = (event.clientX - dragStartClient.x) * dpr;
    const dy = (event.clientY - dragStartClient.y) * dpr;
    const dist = Math.abs(dx) + Math.abs(dy);
    if (!moved && dist > DRAG_THRESHOLD) {
      moved = true;
    }
  });

  element.addEventListener('mouseup', (event) => {
    if (event.button !== 0) return;
    const wasDragging = moved;
    dragStartClient = null;
    moved = false;
    if (invoke) {
      // Save the window's current position (whatever AppKit landed it at).
      invoke('end_drag').catch(() => {});
    }
    if (wasDragging) return;
    if (event.detail >= 3) {
      if (pendingSingleClick) { clearTimeout(pendingSingleClick); pendingSingleClick = null; }
      onRapidClick && onRapidClick();
      return;
    }
    if (event.detail === 2) {
      if (pendingSingleClick) { clearTimeout(pendingSingleClick); pendingSingleClick = null; }
      onDoubleClick && onDoubleClick();
      return;
    }
    if (pendingSingleClick) clearTimeout(pendingSingleClick);
    pendingSingleClick = setTimeout(() => {
      pendingSingleClick = null;
      onSingleClick && onSingleClick();
    }, CLICK_DELAY_MS);
  });

  element.addEventListener('contextmenu', (event) => {
    event.preventDefault();
    onRightClick && onRightClick();
  });
}
