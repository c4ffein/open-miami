// The frame command stream's opcode table for the browser test suites: READ
// from web/ops.js — the one JS copy, pinned to src/graphics.rs by `cargo
// test`. web/ops.js is an ES module and these suites are CommonJS, so its
// `["NAME", args]` rows are parsed from disk rather than imported.
//
// NEVER paste the arity array into a test (it silently desyncs the stream walk
// the day an opcode changes — it happened): require this, and hand
// `OP` / `OP_ARGS` to the page as an argument of `page.evaluate(fn, arg)` /
// `page.addInitScript(fn, arg)`.
const fs = require('fs');
const path = require('path');

const rows = [...fs
  .readFileSync(path.join(__dirname, '../../web/ops.js'), 'utf8')
  .matchAll(/^\s*\["([A-Z_]+)",\s*(\d+)\]/gm)];
if (rows.length === 0) throw new Error('tests/e2e/ops.js: no opcode rows found in web/ops.js');

/** Argument count per opcode (index = opcode). */
const OP_ARGS = rows.map((r) => Number(r[2]));
/** Opcode values by name: `OP.ROBOT === 11`. */
const OP = Object.fromEntries(rows.map((r, i) => [r[1], i]));

module.exports = { OP, OP_ARGS };
