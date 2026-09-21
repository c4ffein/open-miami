/* =========================================================================
   TEXT — the lazy VT323 glyph atlas, one of renderer.js's self-contained
   subsystems (a factory: it owns its state, and takes the few things it needs
   from the batch core once — `quad` / `setTexture` / `flush` are plain
   function references here, exactly as cheap to call as inside the closure).

   VT323 ("GameFont") glyphs are rasterized on demand with a 2D canvas into
   one atlas texture at GLYPH_FS px and drawn as scaled quads through the
   batch. Pixel test: tests/e2e/render/text-glyphs.js.
   ========================================================================= */

/* ---- glyph atlas config ------------------------------------------------- */
const GLYPH_FS = 48; // rasterization font size; quads scale from this
const GLYPH_PAD = 2; // padding inside each glyph cell
const GLYPH_ATLAS_SIZE = 1024;

export function createText({ gl, makeTexture, flush, setTexture, quad }) {
  const glyphTex = makeTexture(GLYPH_ATLAS_SIZE);

  /* ---- glyph atlas: lazy VT323 rasterization ---- */
  const glyphs = new Map(); // char -> {u0,v0,u1,v1,w,h,advance}
  const glyphCellH = Math.ceil(GLYPH_FS * 1.3);
  const glyphBaseline = GLYPH_FS; // baseline offset from cell top
  let glyphPenX = 0;
  let glyphPenY = 0;
  const scratch = document.createElement("canvas");
  const scratchCtx = scratch.getContext("2d", { willReadFrequently: false });

  function bakeGlyph(ch) {
    scratchCtx.font = `${GLYPH_FS}px 'GameFont', monospace`;
    const advance = scratchCtx.measureText(ch).width;
    const cellW = Math.ceil(advance) + GLYPH_PAD * 2;
    if (glyphPenX + cellW > GLYPH_ATLAS_SIZE) {
      glyphPenX = 0;
      glyphPenY += glyphCellH;
    }
    if (glyphPenY + glyphCellH > GLYPH_ATLAS_SIZE) {
      // Atlas full (would need hundreds of distinct glyphs — the game never
      // gets there) — reset it, and ZERO it. The atlas is sampled LINEAR and
      // fallback glyphs (anything VT323 lacks) have other cell widths, so a
      // new generation does not lay out on the old grid: without the clear,
      // stale ink ends up right against a fresh cell and bleeds into its
      // edge texels (measured with real system fonts: up to 54 levels off;
      // tests/e2e/render/text-glyphs.js). Queued quads still point at the old
      // layout: they go out first.
      flush();
      glyphs.clear();
      glyphPenX = 0;
      glyphPenY = 0;
      gl.bindTexture(gl.TEXTURE_2D, glyphTex);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, GLYPH_ATLAS_SIZE, GLYPH_ATLAS_SIZE, 0,
        gl.RGBA, gl.UNSIGNED_BYTE, null);
    }
    scratch.width = cellW;
    scratch.height = glyphCellH;
    scratchCtx.clearRect(0, 0, cellW, glyphCellH);
    scratchCtx.font = `${GLYPH_FS}px 'GameFont', monospace`;
    scratchCtx.fillStyle = "#ffffff";
    scratchCtx.textBaseline = "alphabetic";
    scratchCtx.fillText(ch, GLYPH_PAD, glyphBaseline);
    flush(); // texture upload must not reorder past pending quads
    gl.bindTexture(gl.TEXTURE_2D, glyphTex);
    gl.texSubImage2D(gl.TEXTURE_2D, 0, glyphPenX, glyphPenY, gl.RGBA, gl.UNSIGNED_BYTE, scratch);
    const info = {
      u0: glyphPenX / GLYPH_ATLAS_SIZE,
      v0: glyphPenY / GLYPH_ATLAS_SIZE,
      u1: (glyphPenX + cellW) / GLYPH_ATLAS_SIZE,
      v1: (glyphPenY + glyphCellH) / GLYPH_ATLAS_SIZE,
      w: cellW,
      h: glyphCellH,
      advance,
    };
    glyphs.set(ch, info);
    glyphPenX += cellW;
    return info;
  }

  function drawText(text, x, y, size, r, g, b, a) {
    const s = size / GLYPH_FS;
    let pen = x;
    for (const ch of text) {
      if (ch === " ") {
        let info = glyphs.get(" ");
        if (!info) info = bakeGlyph(" ");
        pen += info.advance * s;
        continue;
      }
      let info = glyphs.get(ch);
      if (!info) info = bakeGlyph(ch);
      setTexture(glyphTex);
      quad(
        pen - GLYPH_PAD * s,
        y - glyphBaseline * s,
        info.w * s,
        info.h * s,
        info.u0, info.v0, info.u1, info.v1,
        r, g, b, a
      );
      pen += info.advance * s;
    }
  }

  return { drawText };
}
