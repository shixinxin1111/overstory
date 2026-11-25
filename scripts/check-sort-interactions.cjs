// Exercise the actual pointer/keyboard handlers without starting a browser
// or changing application data. This does not replace native visual QA.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const ts = require('typescript');

const source = ts.transpileModule(fs.readFileSync('src/shared/sortable/Sortable.tsx', 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX },
}).outputText;

function setup() {
  const cleanups = [];
  let context;
  const events = () => {
    const listeners = new Map();
    return {
      addEventListener(type, fn) { if (!listeners.has(type)) listeners.set(type, new Set()); listeners.get(type).add(fn); },
      removeEventListener(type, fn) { listeners.get(type)?.delete(fn); },
      emit(type, fields = {}) {
        const event = { preventDefault() {}, stopImmediatePropagation() {}, ...fields };
        for (const fn of [...(listeners.get(type) ?? [])]) fn(event);
      },
      count(type) { return listeners.get(type)?.size ?? 0; },
    };
  };
  const window = events(), document = events();
  const jsx = (type, props) => ({ type, props });
  const hooks = {
    createContext: () => (context = { Provider: 'provider', value: null }),
    useContext: c => c.value,
    useRef: current => ({ current }),
    useState: value => [value, () => {}],
    useEffect: fn => cleanups.push(fn()),
  };
  const exports = {};
  vm.runInNewContext(source, {
    exports, window, document, performance, setTimeout, clearTimeout,
    requestAnimationFrame: () => 1, cancelAnimationFrame() {},
    require(name) {
      if (name === 'react') return hooks;
      if (name === 'react/jsx-runtime') return { jsx, jsxs: jsx };
      if (name === 'react-dom') return { createPortal: jsx };
      if (name === 'motion/react') return { motion: { div: 'div' }, useMotionValue: () => ({ set() {} }) };
      if (name === '../motion') return { springs: {} };
      throw new Error(`Unexpected import: ${name}`);
    },
  });
  const rect = (top, height = 40) => ({ left: 0, right: 240, top, bottom: top + height, width: 240, height });
  const element = (dataset, top) => ({
    dataset, captured: false,
    getBoundingClientRect: () => rect(top),
    closest(selector) { return selector === '[data-sort-id]' ? this : null; },
    hasAttribute(name) { return name === 'data-sort-selector' && this.dataset.sortSelector !== undefined; },
    setPointerCapture() { this.captured = true; },
    hasPointerCapture() { return this.captured; },
    releasePointerCapture() { this.captured = false; },
  });
  const rowA = element({ sortId: 'a', sortKind: 'item', sortGroup: 'one' }, 0);
  const rowB = element({ sortId: 'b', sortKind: 'item', sortGroup: 'one' }, 50);
  const empty = element({ sortDrop: 'two' }, 100);
  const hits = [rowA, rowB, empty];
  const moves = [];
  const tree = exports.SortableScope({
    children: null, groups: [{ id: 'one', name: '第一组' }, { id: 'two', name: '第二组' }],
    onMove: async (entry, target) => moves.push({ entry, target }),
  });
  context.value = tree.props.value;
  tree.props.children[0].props.ref.current = { querySelectorAll: selector => selector === '[data-sort-id]' ? [rowA, rowB] : hits };
  const entry = { id: 'a', kind: 'item', label: '第一项', groupId: 'one' };
  const row = exports.SortRow({ entry, children: null });
  const target = (kind = 'row') => ({
    closest(selector) {
      if (selector === 'button') return kind === 'button' || kind === 'action' ? { hasAttribute: attr => kind === 'button' && attr === 'data-row-drag' } : null;
      return ['input', 'textarea', 'checkbox', 'action'].includes(kind) ? {} : null;
    },
  });
  const down = (kind = 'button') => {
    const e = { button: 0, clientX: 20, clientY: 20, pointerId: 1, currentTarget: rowA, target: target(kind), prevented: false, preventDefault() { this.prevented = true; }, stopPropagation() {} };
    row.props.onPointerDown(e);
    return e;
  };
  const move = (y, x = 20) => window.emit('pointermove', { clientX: x, clientY: y });
  const click = () => {
    const e = { detail: 1, stopped: false, preventDefault() {}, stopPropagation() { this.stopped = true; } };
    row.props.onClickCapture(e); return e.stopped;
  };
  const key = value => row.props.onKeyDown({ key: value, target: rowA, currentTarget: rowA, preventDefault() {}, stopPropagation() {} });
  return { exports, row, rowA, entry, down, move, click, key, window, document, moves, close: () => cleanups.forEach(fn => fn?.()) };
}

function check(name, fn) {
  const env = setup();
  try { fn(env); console.log(`PASS ${name}`); } finally { env.close(); }
}

check('ordinary click and small movement preserve button events', e => {
  assert.equal(e.down().prevented, false);
  assert.equal(e.rowA.captured, false);
  e.move(23);
  assert.equal(e.rowA.captured, false);
  e.window.emit('pointerup');
  assert.equal(e.click(), false);
  assert.equal(e.moves.length, 0);
});

for (const kind of ['input', 'textarea', 'checkbox', 'action']) {
  check(`${kind} never starts sorting`, e => {
    e.down(kind);
    assert.equal(e.window.count('pointermove'), 0);
    e.move(60);
    e.window.emit('pointerup');
    assert.equal(e.moves.length, 0);
  });
}

check('whole-row drag commits order and suppresses the release click', e => {
  e.down(); e.move(80);
  assert.equal(e.rowA.captured, true);
  e.window.emit('pointerup');
  assert.equal(e.rowA.captured, false);
  assert.deepEqual(JSON.parse(JSON.stringify(e.moves[0].target)), { groupId: 'one', beforeId: null });
  assert.equal(e.click(), true);
  e.down('action');
  assert.equal(e.click(), false);
});

check('drag into an empty group keeps its destination', e => {
  e.down(); e.move(120); e.window.emit('pointerup');
  assert.deepEqual(JSON.parse(JSON.stringify(e.moves[0].target)), { groupId: 'two', beforeId: null });
});

check('Escape cancels pointer sorting without a mutation', e => {
  e.down(); e.move(80); e.document.emit('keydown', { key: 'Escape' }); e.window.emit('pointerup');
  assert.equal(e.moves.length, 0);
  assert.equal(e.window.count('pointermove'), 0);
  assert.equal(e.rowA.captured, false);
});

check('disabled search results cannot sort', e => {
  const disabled = e.exports.SortRow({ entry: e.entry, children: null, disabled: true });
  disabled.props.onPointerDown({ button: 0 });
  assert.equal(e.window.count('pointermove'), 0);
  assert.equal(disabled.props.tabIndex, undefined);
});

check('keyboard sorting moves across groups and commits once', e => {
  e.key(' '); e.key('ArrowRight'); e.key('Enter');
  assert.equal(e.moves.length, 1);
  assert.deepEqual(JSON.parse(JSON.stringify(e.moves[0].target)), { groupId: 'two', beforeId: null });
});
