import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import vm from 'node:vm';

async function setup() {
  const handlers = {}, timers = new Map(), actions = [], commands = [], captured = new Set();
  let next = 0;
  const element = {
    addEventListener: (name, fn) => handlers[name] = fn,
    setPointerCapture: id => captured.add(id),
    hasPointerCapture: id => captured.has(id),
    releasePointerCapture: id => captured.delete(id),
  };
  const context = vm.createContext({ window: { __TAURI__: { core: { invoke: name => { commands.push(name); return Promise.resolve(); } } }, addEventListener: (n, f) => handlers[n] = f }, setTimeout: fn => { timers.set(++next, fn); return next; }, clearTimeout: id => timers.delete(id), Math });
  vm.runInContext((await fs.readFile(new URL('../ui/mouse.js', import.meta.url), 'utf8')).replace('export function', 'function') + '\nthis.install = installMouseHandling;', context);
  context.install(element, Object.fromEntries(['SingleClick', 'DoubleClick', 'RapidClick', 'RightClick'].map(n => ['on'+n, () => actions.push(n)])));
  const send = (name, extra = {}) => handlers[name]({ button: 0, pointerId: 1, screenX: 100, screenY: 100, detail: 0, preventDefault() {}, ...extra });
  const click = () => { send('pointerdown'); send('pointerup'); };
  const flush = () => { for (const [id, fn] of [...timers]) { timers.delete(id); fn(); } };
  return { send, click, flush, actions, commands };
}
for (const [count, expected] of [[1,'SingleClick'],[2,'DoubleClick'],[3,'RapidClick'],[5,'RapidClick']]) {
  test(`${count} completed clicks with WebView detail=0 trigger only ${expected}`, async () => {
    const s = await setup(); for(let i=0;i<count;i++) s.click(); s.flush();
    assert.deepEqual(s.actions,[expected]); assert.deepEqual(s.commands,[]);
  });
}
test('drag begins only past threshold and suppresses click', async () => {
  const s = await setup(); s.send('pointerdown'); s.send('pointermove',{screenX:102}); assert.deepEqual(s.commands,[]);
  s.send('pointermove',{screenX:110}); s.send('pointerup'); s.flush();
  assert.equal(s.commands[0],'native_drag'); assert.deepEqual(s.actions,[]);
});
test('cancel, focus loss and right-click cancel pending left clicks', async () => {
  for (const name of ['pointercancel','blur','contextmenu']) {
    const s=await setup();s.click();s.send(name);s.flush();assert.deepEqual(s.actions,name==='contextmenu'?['RightClick']:[]);
  }
});
