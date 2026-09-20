/* =========================================================================
   OPEN MIAMI - the frame command stream's OPCODE TABLE, JS side.

   The wasm engine records each frame as a flat f32 stream: an opcode followed
   by its fixed number of f32 arguments (src/graphics.rs `mod op`; ops 21-23
   in src/static_geo.rs). This file is the ONE JS copy of that table:
   renderer.js walks the stream with it, gpu-probe.js rewrites streams with
   it, and the e2e helpers (tests/e2e/specs/helpers.js) read it from disk.

   Dependency-free on purpose (no imports, no DOM) so anything can load it.

   PINNED TO RUST by `cargo test` (src/graphics/stream.rs): every row's name,
   position (= opcode value) and arity must match `mod op` / `OP_ARGS`, and
   every `case N: // NAME` of renderer.js's dispatch must agree with it. Add
   an opcode = a row here + the Rust side + the renderer's `case`.
   ========================================================================= */

// [name, argument count]  — the ROW INDEX is the opcode value.
const TABLE = [
  ["CLEAR", 4],        //  0  r g b a
  ["RECT", 8],         //  1  x y w h  r g b a
  ["RECT_LINES", 9],   //  2  x y w h thickness  r g b a
  ["CIRCLE", 7],       //  3  x y radius  r g b a
  ["LINE", 9],         //  4  x1 y1 x2 y2 thickness  r g b a
  ["ARC", 9],          //  5  x y radius a0 a1  r g b a  (filled pie)
  ["TEXT", 8],         //  6  textIdx x y size  r g b a
  ["SAVE", 0],         //  7
  ["RESTORE", 0],      //  8
  ["TRANSLATE", 2],    //  9  x y
  ["ROTATE", 1],       // 10  angle
  ["ROBOT", 18],       // 11  colorIdx weaponIdx flags x y angle sizePx + 11 pose scalars
  ["SCALE", 2],        // 12  sx sy
  ["SHOGGOTH", 4],     // 13  x y sizePx maskAt  (consumes the SPHERE run before it)
  ["POSTFX", 5],       // 14  kind t r g b
  ["PIX_BEGIN", 4],    // 15  px w h smooth
  ["PIX_END", 2],      // 16  x y
  ["PORTRAIT", 18],    // 17  colorIdx x y sizePx time mode + 11 pose scalars + flags
  ["GUN_PICKUP", 5],   // 18  weaponIdx x y angle sizePx
  ["PIX_BLIT", 6],     // 19  sx sy sw sh x y
  ["DRIVE", 16],       // 20  w h t glitch split px dim o0..o8
  ["STATIC_BEGIN", 1], // 21  key
  ["STATIC_END", 0],   // 22
  ["STATIC_REF", 1],   // 23  key
  ["BACKDROP", 8],     // 24  w h t px ex ey ew eh
  ["HEAD", 5],         // 25  colorIdx x y angle sizePx
  ["SPHERE", 20],      // 26  model rows 0..2 (12)  r g b id  ar ag ab emission
];

/** Opcode values by name: `OP.RECT === 1`. */
export const OP = Object.freeze(Object.fromEntries(TABLE.map(([name], i) => [name, i])));

/** Argument count per opcode (index = opcode) — what a stream walk steps by. */
export const OP_ARGS = Object.freeze(TABLE.map(([, args]) => args));

/** Separator between entries of the per-frame text arena (never in game text). */
export const TEXT_SEP = "\u001f";
