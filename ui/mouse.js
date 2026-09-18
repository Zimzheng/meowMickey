const tauri = window.__TAURI__;
const invoke = tauri?.core?.invoke || tauri?.invoke;
const DRAG_THRESHOLD = 3;
const CLICK_DELAY_MS = 250;

export function installMouseHandling(element, { onSingleClick, onDoubleClick, onRightClick }) {
  let dragStartClient = null;
  let dragOrigin = null;
  let moved = false;
  let pendingSingleClick = null;

  element.addEventListener('mousedown', (event) => {
    if (event.button !== 0) return; // left button only for drag
    dragStartClient = { x: event.clientX, y: event.clientY };
    moved = false;
    if (invoke) {
      invoke('start_drag').then((origin) => {
        dragOrigin = origin;
      }).catch(() => { dragOrigin = null; });
    } else {
      dragOrigin = { x: 0, y: 0 };
    }
  });

  element.addEventListener('mousemove', (event) => {
    if (!dragStartClient) return;
    const dx = event.clientX - dragStartClient.x;
    const dy = event.clientY - dragStartClient.y;
    if (!moved && Math.abs(dx) + Math.abs(dy) > DRAG_THRESHOLD) {
      moved = true;
    }
    if (moved && dragOrigin && invoke) {
      invoke('update_drag', { x: dx, y: dy }).catch(() => {});
    }
  });

  element.addEventListener('mouseup', (event) => {
    if (event.button !== 0) return;
    const wasDragging = moved;
    const wasClick = !moved;
    dragStartClient = null;
    dragOrigin = null;
    moved = false;

    if (invoke) {
      invoke('end_drag').catch(() => {});
    }

    if (wasDragging) return;

    if (event.detail >= 2) {
      // Double-click (browser detected via detail)
      if (pendingSingleClick) {
        clearTimeout(pendingSingleClick);
        pendingSingleClick = null;
      }
      onDoubleClick && onDoubleClick();
      return;
    }

    // Single click with manual double-click window
    if (pendingSingleClick) {
      clearTimeout(pendingSingleClick);
    }
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