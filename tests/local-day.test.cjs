const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const ts = require('typescript');

const source = ts.transpileModule(
  fs.readFileSync('src/composables/useLocalDay.ts', 'utf8'),
  { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } },
).outputText;

function setup(refresh = async () => true) {
  let now = new Date(2026, 10, 13, 23, 59, 59).getTime();
  let mount;
  let unmount;
  let nextId = 0;
  const timers = new Map();
  const calls = [];
  const window = new EventTarget();
  const document = new EventTarget();
  document.visibilityState = 'visible';
  class ClockDate extends Date {
    constructor(...args) { super(...(args.length ? args : [now])); }
  }
  const exports = {};
  vm.runInNewContext(source, {
    exports, Date: ClockDate, window, document, console,
    require: () => ({
      ref: (value) => ({ value }),
      onMounted: (fn) => { mount = fn; },
      onUnmounted: (fn) => { unmount = fn; },
    }),
    setTimeout: (fn, delay) => { timers.set(++nextId, { fn, delay, repeat: false }); return nextId; },
    setInterval: () => { throw new Error('Date refresh must not poll'); },
    clearTimeout: (id) => timers.delete(id),
  });
  const today = exports.useLocalDay(async (previous) => {
    calls.push(previous);
    return refresh();
  });
  mount();
  return {
    today, calls, timers, window, document, unmount,
    advanceTo: (...args) => { now = new Date(...args).getTime(); },
    fire: async () => {
      const [id, timer] = [...timers][0];
      timers.delete(id);
      await timer.fn();
      await new Promise(setImmediate);
    },
    settle: () => new Promise(setImmediate),
  };
}

test('local midnight refreshes once and schedules the following midnight', async () => {
  const state = setup();
  assert.equal(state.today.value, '2026-11-13');
  assert.equal([...state.timers.values()].find((t) => !t.repeat).delay, 1000);
  state.advanceTo(2026, 10, 14);
  await state.fire();
  assert.equal(state.today.value, '2026-11-14');
  assert.deepEqual(state.calls, ['2026-11-13']);
  assert.equal([...state.timers.values()].find((t) => !t.repeat).delay, 86_400_000);
  state.window.dispatchEvent(new Event('focus'));
  await state.settle();
  assert.equal(state.calls.length, 1);
  assert.equal(state.timers.size, 1);
  state.unmount();
});

test('delayed midnight timer catches several suspended days across year boundary', async () => {
  const state = setup();
  state.advanceTo(2027, 0, 2, 8);
  await state.fire();
  assert.equal(state.today.value, '2027-01-02');
  assert.equal(state.calls.length, 1);
  assert.equal([...state.timers.values()][0].delay, 16 * 60 * 60 * 1000);
  state.unmount();
});

test('focus and visibility catch up without duplicate concurrent reloads', async () => {
  let finish;
  const state = setup(() => new Promise((resolve) => { finish = resolve; }));
  state.advanceTo(2026, 10, 14, 8);
  state.window.dispatchEvent(new Event('focus'));
  state.document.dispatchEvent(new Event('visibilitychange'));
  assert.equal(state.calls.length, 1);
  finish(true);
  await state.settle();
  state.window.dispatchEvent(new Event('focus'));
  assert.equal(state.calls.length, 1);
  state.unmount();
});

test('failed refresh retries on the next check', async () => {
  let success = false;
  const state = setup(async () => success);
  state.advanceTo(2026, 10, 14);
  await state.fire();
  success = true;
  state.window.dispatchEvent(new Event('focus'));
  await state.settle();
  state.document.dispatchEvent(new Event('visibilitychange'));
  await state.settle();
  assert.deepEqual(state.calls, ['2026-11-13', '2026-11-13']);
  state.unmount();
});

test('unmount removes timers and event listeners', async () => {
  const state = setup();
  state.unmount();
  state.advanceTo(2026, 10, 14);
  state.window.dispatchEvent(new Event('focus'));
  state.document.dispatchEvent(new Event('visibilitychange'));
  await state.settle();
  assert.equal(state.timers.size, 0);
  assert.equal(state.calls.length, 0);
});
