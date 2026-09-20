/* =========================================================================
   OPEN MIAMI - the renderer's GLSL sources (WebGL 1). Pure data: no shader
   text is built at runtime (renderer.js prepends `#define`s where a variant
   exists, e.g. the opt-in grain fold). Kept apart so renderer.js reads as
   the pipeline and a shader edit is a one-file diff.
   ========================================================================= */

// uXA/uXB: a 2D affine (rows [a c e] / [b d f]) applied to aPos before the
// resolution mapping. Identity for every dynamic draw (the CPU tessellation
// already applied the transform stack); set to the camera transform only
// while drawing the STATIC geometry cache, whose VBO holds raw world
// coordinates — that is what lets one baked buffer track a moving camera.
export const VS = `
attribute vec2 aPos;
attribute vec2 aUv;
attribute vec4 aColor;
uniform vec2 uRes;
uniform vec3 uXA;
uniform vec3 uXB;
varying vec2 vUv;
varying vec4 vColor;
void main(){
  vUv = aUv;
  vColor = aColor;
  vec3 p = vec3(aPos, 1.0);
  vec2 t = vec2(dot(uXA, p), dot(uXB, p));
  gl_Position = vec4(t.x / uRes.x * 2.0 - 1.0, 1.0 - t.y / uRes.y * 2.0, 0.0, 1.0);
}
`;

/* THE FOLDED TV STATIC (uGrainT > 0). The film grain used to be one more
   alpha-blended full-screen quad over the finished frame — a whole layer of
   fill (1.8 ms of a 16.7 ms frame on a 2018 MacBook Air) for a 7.5% effect.
   Blending the noise texel n (straight alpha, opacity k = n.a * t) over a
   colour c is the AFFINE map  g(c) = c*(1-k) + n.rgb*k , and an affine map
   with per-pixel constants commutes with alpha blending:
       g(s)*a + g(d)*(1-a) = g(s*a + d*(1-a))
   so if EVERY fragment that reaches the canvas is grained as it is written
   (same n, k for a given screen pixel: looked up by gl_FragCoord), the final
   pixel is the grained final colour — what the quad produced, minus the
   layer (to within 8-bit rounding: the quad rounded once, this rounds per
   blend). For a PREMULTIPLIED source (a pixel group's composite, blended
   ONE / 1-a) the same identity needs  g(p) = p*(1-k) + n.rgb*k*a .
   Conditions, enforced by frameRender: the frame opens with an opaque
   full-screen layer (the BACKDROP — the clear colour itself is never
   grained), only draws that land ON THE CANVAS are grained (never the inside
   of a pixel group or the scene FBO), and no post pass follows.

   OPT-IN (`?grain=fold`), NOT the default. The pixels are right
   (tests/e2e/grain-fold.js); the ECONOMICS are a trade, measured on a 2018
   MacBook Air with `?gpuprobe=headroom` (4.12 Mpx canvas): the fold removes
   the quad's layer (game frame 8.6 -> 6.8 ms of GPU time) but its second
   texture fetch makes EVERY batch fragment ~47% dearer (a full-screen batch
   layer 1.44 -> 2.11 ms), so the frame tolerates FEWER extra layers (5.6 ->
   4.7) even though it has more ms to spare: break-even is ~2 full-screen
   layers of batch fill per frame — a win for today's scenes, a loss for a
   busier one. (An earlier verdict of "a clear loss, fragments twice as
   dear" was taken with the probe's panel over the canvas, which distorted
   every figure — see CLAUDE.md, GPU probe.) With ~8 ms of headroom either
   way it stays off: the plain shader scales better and is simpler. Without
   the flag the shader is compiled WITHOUT the grain code: byte-for-byte the
   plain textured-quad shader. */
export const FS = `
precision mediump float;
varying vec2 vUv;
varying vec4 vColor;
uniform sampler2D uTex;
#ifdef GRAIN
uniform sampler2D uGrain;
uniform float uGrainT;    // 0 = off, else the static's opacity
uniform float uGrainPre;  // 1 = this draw's colour is premultiplied
#ifdef GL_FRAGMENT_PRECISION_HIGH
uniform highp vec4 uGrainK; // xy = uv per physical px (y negative: v runs top-down), zw = this frame's offset
#else
uniform vec4 uGrainK;
#endif
#endif
void main(){
  vec4 c = texture2D(uTex, vUv) * vColor;
#ifdef GRAIN
  if (uGrainT > 0.0) {
    vec4 n = texture2D(uGrain, gl_FragCoord.xy * uGrainK.xy + uGrainK.zw);
    float k = n.a * uGrainT;
    c.rgb = c.rgb * (1.0 - k) + n.rgb * (k * mix(1.0, c.a, uGrainPre));
  }
#endif
  gl_FragColor = c;
}
`;

export const POST_VS = `
attribute vec2 aPos;
varying vec2 vUv;
void main(){
  vUv = aPos * 0.5 + 0.5;
  gl_Position = vec4(aPos, 0.0, 1.0);
}
`;

// Full-screen post pass. One shader, one uniform selecting the look — the
// kinds are all cheap single-pass tricks (a few extra taps at most), kept
// deliberately dependency-free. See the header table for the kind list.
// highp (when available): screen-pixel arithmetic in the 1000s that fp16
// cannot represent (the same guard as robot-core's post pass).
export const POST_FS = `
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
varying vec2 vUv;
uniform sampler2D uScene;
uniform vec2 uRes;
uniform float uKind;
uniform float uT;
uniform vec3 uColor;
uniform float uTime;

float hash(vec2 p) {
  return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

// Hue rotation: Rodrigues rotation of the rgb vector about the gray axis.
vec3 hueShift(vec3 color, float a) {
  const vec3 k = vec3(0.57735);
  float ca = cos(a);
  return color * ca + cross(k, color) * sin(a) + k * dot(k, color) * (1.0 - ca);
}

float luma(vec3 c) {
  return dot(c, vec3(0.299, 0.587, 0.114));
}

// Chromatic split sample: r/b pulled apart along +-off.
vec3 splitSample(vec2 uv, vec2 off) {
  return vec3(
    texture2D(uScene, uv + off).r,
    texture2D(uScene, uv).g,
    texture2D(uScene, uv - off).b
  );
}

void main(){
  vec2 uv = vUv;
  vec3 c;
  float t = clamp(uT, 0.0, 1.0);
  // Coarse, time-jittered noise cell (grain / dissolve dither).
  float n = hash(floor(uv * uRes / 3.0) + floor(uTime * 24.0) * 0.371);
  float scan = 0.5 + 0.5 * sin(uv.y * uRes.y * 3.14159);
  if (uKind < 0.5) {
    // ---- 0 BLUR-OUT: two rings of taps whose radius grows with t ----
    float radPx = t * t * 34.0 + t * 2.0;
    vec2 px = radPx / uRes;
    vec3 acc = texture2D(uScene, uv).rgb * 2.0;
    float wsum = 2.0;
    for (int i = 0; i < 8; i++) {
      float a = float(i) * 0.785398 + uTime * 0.7;
      vec2 d = vec2(cos(a), sin(a));
      acc += texture2D(uScene, uv + d * px).rgb;
      acc += texture2D(uScene, uv + d * px * 0.5).rgb * 1.5;
      wsum += 2.5;
    }
    c = acc / wsum;
    // Dissolve toward the colour, dithered by the grain so it eats in patches.
    float k = smoothstep(0.12, 1.0, t + (n - 0.5) * 0.35 * t);
    c = mix(c, uColor, k);
    c *= 1.0 - 0.22 * t * scan;
    c += (n - 0.5) * 0.12 * t;
  } else if (uKind < 1.5) {
    // ---- 1 SYNTHWAVE CRT: chromatic split, scanlines, vignette, grain ----
    c = splitSample(uv, vec2(1.6 * t / uRes.x, 0.0));
    c *= 1.0 - 0.28 * t * scan;
    vec2 q = uv * (1.0 - uv);
    float vig = pow(clamp(q.x * q.y * 18.0, 0.0, 1.0), 0.28 * t);
    c = c * vig + uColor * 0.10 * t * (1.0 - vig);
    c += (n - 0.5) * 0.06 * t;
  } else if (uKind < 2.5) {
    // ---- 2 VHS TAPE: tracking band, line jitter, chroma bleed, dropouts ----
    // A tracking band rolls up the screen; lines inside it tear hard.
    float yb = fract(uTime * 0.13);
    float db = abs(uv.y - yb);
    float band = smoothstep(0.045, 0.0, min(db, 1.0 - db));
    float ln = floor(uv.y * uRes.y);
    float jit = (hash(vec2(ln, floor(uTime * 24.0))) - 0.5)
      * (4.0 + band * 90.0) * t / uRes.x;
    vec2 suv = vec2(uv.x + jit + band * 0.02 * t * sin(uTime * 43.0 + uv.y * 61.0), uv.y);
    c = splitSample(suv, vec2(2.5 * t / uRes.x, 0.0));
    // Washed-out tape colour, whitened noise inside the band.
    c = mix(c, vec3(luma(c)), 0.25 * t);
    c += band * t * (0.18 + 0.45 * n);
    // Rare white dropout streaks.
    float drop = step(0.994, hash(vec2(ln, floor(uTime * 60.0) + 7.0)));
    c = mix(c, vec3(0.9), drop * 0.8 * t);
    // Head-switch noise bar pinned to the bottom edge.
    c = mix(c, vec3(n), step(0.972, uv.y) * 0.5 * t);
    c *= 1.0 - 0.18 * t * scan;
    c += (n - 0.5) * 0.10 * t;
  } else if (uKind < 3.5) {
    // ---- 3 DRUNK SWAY: rotation/zoom breathing, wavy warp, ghost, hue ----
    float asp = uRes.x / uRes.y;
    vec2 p = uv - 0.5;
    p.x *= asp;
    float ang = (sin(uTime * 0.8) * 0.045 + sin(uTime * 0.47 + 1.7) * 0.030) * t;
    float ca = cos(ang), sa = sin(ang);
    p = vec2(p.x * ca - p.y * sa, p.x * sa + p.y * ca);
    p /= 1.0 + (0.05 + 0.03 * sin(uTime * 1.1)) * t;
    p.x /= asp;
    vec2 wuv = p + 0.5;
    wuv += vec2(sin(wuv.y * 7.0 + uTime * 1.3), cos(wuv.x * 6.0 + uTime * 1.1)) * 0.006 * t;
    vec3 base = texture2D(uScene, wuv).rgb;
    // Double-vision ghost slowly orbiting the true image.
    vec2 gof = vec2(cos(uTime * 0.6), sin(uTime * 0.45)) * 9.0 * t / uRes;
    vec3 ghost = texture2D(uScene, wuv + gof).rgb;
    c = mix(base, max(base, ghost), 0.5 * t);
    c = hueShift(c, 0.5 * t * sin(uTime * 0.5));
    c *= 1.0 - 0.10 * t * scan;
    c += (n - 0.5) * 0.05 * t;
  } else if (uKind < 4.5) {
    // ---- 4 CRT TUBE: barrel distortion, aperture grille, flicker ----
    vec2 p = uv * 2.0 - 1.0;
    float r2 = dot(p, p);
    p *= 1.0 + 0.12 * t * r2;
    vec2 cuv = p * 0.5 + 0.5;
    // Off-tube pixels go black (the bezel).
    float inb = step(0.0, cuv.x) * step(cuv.x, 1.0) * step(0.0, cuv.y) * step(cuv.y, 1.0);
    c = splitSample(cuv, vec2(1.2 * t * (1.0 + r2) / uRes.x, 0.0));
    // Aperture grille: RGB phosphor triads across x.
    float px3 = mod(floor(cuv.x * uRes.x), 3.0);
    vec3 tri = vec3(step(px3, 0.5), step(0.5, px3) * step(px3, 1.5), step(1.5, px3));
    c *= mix(vec3(1.0), tri * 1.9 + 0.25, 0.7 * t);
    float scan2 = 0.5 + 0.5 * sin(cuv.y * uRes.y * 3.14159);
    c *= 1.0 - 0.35 * t * scan2;
    c *= 1.0 - 0.04 * t * (0.5 + 0.5 * sin(uTime * 87.0)); // mains flicker
    vec2 q = cuv * (1.0 - cuv);
    c *= pow(clamp(q.x * q.y * 25.0, 0.0, 1.0), 0.45 * t) * inb;
    c += (n - 0.5) * 0.05 * t * inb;
  } else if (uKind < 5.5) {
    // ---- 5 ACID TRIP: radial hue cycling, oversaturate, posterize ----
    vec2 wuv = uv + vec2(sin(uv.y * 12.0 + uTime * 1.7), cos(uv.x * 11.0 + uTime * 1.3)) * 0.004 * t;
    c = texture2D(uScene, wuv).rgb;
    float r = length(uv - 0.5);
    c = hueShift(c, t * (uTime * 1.2 + r * 6.0));
    c = mix(vec3(luma(c)), c, 1.0 + 0.9 * t); // oversaturate
    c = mix(c, floor(c * 6.0 + 0.5) / 6.0, 0.5 * t); // mild posterize
    c *= 1.0 - 0.10 * t * scan;
    c += (n - 0.5) * 0.05 * t;
  } else if (uKind < 6.5) {
    // ---- 6 DATAMOSH: slice/block displacement, channel swap, noise ----
    float rt = floor(uTime * 12.0);
    float seg = floor(uv.y * 28.0);
    float r1 = hash(vec2(seg, rt));
    float tear = step(0.72, r1);
    float shift = (r1 - 0.5) * 0.22 * t * tear;
    vec2 blk = floor(uv * vec2(12.0, 8.0));
    float br = hash(blk + rt * 0.13);
    shift += (hash(blk + rt) - 0.5) * 0.2 * t * step(0.93, br);
    vec2 guv = vec2(fract(uv.x + shift), uv.y);
    c = splitSample(guv, vec2((4.0 + 10.0 * tear) * t / uRes.x, 0.0));
    // Corrupted blocks: swapped channels or raw digital noise.
    c = mix(c, c.gbr, step(0.965, br) * t);
    vec3 noiseCol = vec3(hash(blk + rt * 3.7), hash(blk + rt * 5.1), hash(blk + rt * 7.3));
    c = mix(c, noiseCol, step(1.0 - 0.06 * t, hash(blk + rt + 31.0)));
    c *= 1.0 - 0.12 * t * scan;
    c += (n - 0.5) * 0.08 * t;
  } else if (uKind < 7.5) {
    // ---- 7 NEON BLOOM: bright-pass glow + shadow tint toward the colour ----
    c = texture2D(uScene, uv).rgb;
    vec3 glow = vec3(0.0);
    for (int i = 0; i < 8; i++) {
      float a = float(i) * 0.785398;
      vec2 d = vec2(cos(a), sin(a)) * (6.0 / uRes);
      glow += max(texture2D(uScene, uv + d).rgb - 0.45, 0.0);
      glow += max(texture2D(uScene, uv + d * 2.5).rgb - 0.45, 0.0) * 0.6;
    }
    glow /= 12.8;
    c += glow * 2.2 * t * (0.92 + 0.08 * sin(uTime * 9.0));
    c += uColor * 0.12 * t * (1.0 - luma(c)); // lift the shadows into neon
    c *= 1.0 - 0.10 * t * scan;
    c += (n - 0.5) * 0.04 * t;
  } else if (uKind < 8.5) {
    // ---- 8 PIXEL MOSAIC: chunky pixelation + dithered posterize ----
    float cell = 1.0 + 6.0 * t;
    vec2 id = floor(uv * uRes / cell);
    c = texture2D(uScene, (id + 0.5) * cell / uRes).rgb;
    float levels = 5.0;
    float dith = (hash(id) - 0.5) / levels;
    c = mix(c, floor((c + dith) * levels + 0.5) / levels, t);
    c *= 1.0 - 0.08 * t * scan;
  } else if (uKind < 9.5) {
    // ---- 9 TUNNEL RUSH: radial zoom blur toward the centre ----
    vec2 p = uv - 0.5;
    vec3 acc = vec3(0.0);
    float wsum = 0.0;
    for (int i = 0; i < 10; i++) {
      float k = float(i) / 10.0;
      float w = 1.0 - k * 0.8;
      acc += texture2D(uScene, p * (1.0 - 0.22 * t * k) + 0.5).rgb * w;
      wsum += w;
    }
    c = acc / wsum;
    float rr = length(p);
    c *= 1.0 + 0.25 * t * (1.0 - smoothstep(0.0, 0.45, rr)); // hot centre
    vec2 q = uv * (1.0 - uv);
    c *= pow(clamp(q.x * q.y * 18.0, 0.0, 1.0), 0.4 * t);
    c += (n - 0.5) * 0.06 * t;
  } else if (uKind < 11.5) {
    // ---- 11 UI GREY: the modal wash — what's under, desaturated to
    // white/black, tape noise + scanlines + coarse horizontal grain,
    // gently vignetted. (Kind 10 WARP TRAILS never reaches this shader:
    // frameRender routes it to the feedback pass.)
    c = texture2D(uScene, uv).rgb;
    float g = luma(c);
    c = mix(c, vec3(g), 0.85 * t);
    c *= 1.0 - 0.10 * t * scan;
    c += (n - 0.5) * 0.10 * t;
    float ln = hash(vec2(floor(uv.y * uRes.y / 3.0), floor(uTime * 13.0)));
    c += (ln - 0.5) * 0.06 * t;
    vec2 q = uv * (1.0 - uv);
    c *= pow(clamp(q.x * q.y * 20.0, 0.0, 1.0), 0.25 * t);
    c = mix(c, c * uColor, 0.20 * t);
  } else {
    // ---- 12 MODAL STATIC: uColor.r/.g = the centred modal's HALF extents
    // (kind 13 TV STATIC never reaches this shader: frameRender draws it
    // as a plain blended noise quad — no scene pass needed.)
    // as fractions of the screen. INSIDE that rect: the kind-11 grey/tape
    // wash (the modal itself). OUTSIDE: the scene blurred, desaturated and
    // buried under t coverage of hard 6-px binary white noise — real
    // dead-channel static, re-rolled every frame.
    vec2 halfExt = vec2(uColor.r, uColor.g);
    vec2 dc = abs(uv - 0.5);
    // Distance outside the panel edge, in screen px (Chebyshev = square
    // rings): a 6-px WHITE band then a 6-px BLACK band frame the panel
    // before the static starts — one noise-pixel each.
    vec2 opx = max(dc - halfExt, 0.0) * uRes;
    float ring = max(opx.x, opx.y);
    if (dc.x <= halfExt.x && dc.y <= halfExt.y) {
      // The panel: scene passed through untouched (an opaque black fill
      // with white text — no wash, no tint).
      c = texture2D(uScene, uv).rgb;
    } else if (ring <= 6.0) {
      c = vec3(1.0);
    } else if (ring <= 12.0) {
      c = vec3(0.0);
    } else {
      vec2 px = 1.0 / uRes;
      // The 6-px noise grid is ANCHORED TO THE PANEL CORNER, not the
      // screen: with the panel size a multiple of 6, every cell around the
      // rings is whole — no sliced pixels at the edges.
      vec2 originPx = (vec2(0.5) - halfExt) * uRes;
      vec2 cell = floor((gl_FragCoord.xy - originPx) / 6.0);
      // Sinless hash (Hoskins hash13): the sin-based one is a scrambled
      // PLANE WAVE, and at some phases its diagonal ridges de-scramble
      // into visible bands for a moment. This one has no linear structure.
      // frame is a third axis (phase, never a spatial slide), wrapped to
      // keep floats small.
      float frame = mod(floor(uTime * 60.0), 240.0);
      vec3 p3 = fract(vec3(cell, frame) * 0.1031);
      p3 += dot(p3, p3.zyx + 31.32);
      float roll = fract((p3.x + p3.y) * p3.z);
      vec3 q3 = fract(vec3(cell, frame + 61.0) * 0.1031);
      q3 += dot(q3, q3.zyx + 31.32);
      float bw = step(0.5, fract((q3.x + q3.y) * q3.z));
      float cover = step(1.0 - t, roll);
      if (cover > 0.5) {
        // A static cell: 26% the average of the pixels under the whole
        // cell (5 taps spread across its 6x6 px), 74% the random b/w.
        vec2 cc = (originPx + (cell + 0.5) * 6.0) * px;
        vec3 avg = texture2D(uScene, cc).rgb * 0.2;
        avg += texture2D(uScene, cc + vec2( 2.0,  2.0) * px).rgb * 0.2;
        avg += texture2D(uScene, cc + vec2(-2.0,  2.0) * px).rgb * 0.2;
        avg += texture2D(uScene, cc + vec2( 2.0, -2.0) * px).rgb * 0.2;
        avg += texture2D(uScene, cc + vec2(-2.0, -2.0) * px).rgb * 0.2;
        c = 0.42 * avg + 0.58 * vec3(bw);
      } else {
        vec3 b = texture2D(uScene, uv).rgb * 0.28;
        b += texture2D(uScene, uv + vec2(3.0, 0.0) * px).rgb * 0.18;
        b += texture2D(uScene, uv - vec2(3.0, 0.0) * px).rgb * 0.18;
        b += texture2D(uScene, uv + vec2(0.0, 3.0) * px).rgb * 0.18;
        b += texture2D(uScene, uv - vec2(0.0, 3.0) * px).rgb * 0.18;
        float g = luma(b);
        c = mix(b, vec3(g), 0.65);
      }
    }
  }
  gl_FragColor = vec4(c, 1.0);
}
`;

// POSTFX kind 10 (WARP TRAILS) — a separate two-mode feedback shader so the
// single-pass POST_FS above stays byte-for-byte what kinds 0-9 always ran.
//   mode 0 (combine, drawn into the write accumulator): sample the read
//     accumulator with UVs pulled slightly TOWARD the centre — its content
//     therefore appears pushed OUTWARD over time — decay it (tinted by
//     uColor), then stamp in the scene's bright, saturated pixels. `t`
//     drives both the pull distance and the decay.
//   mode 1 (present, drawn to the canvas): the crisp scene screen-blended
//     with the freshly written trails.
export const WARP_FS = `
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
varying vec2 vUv;
uniform sampler2D uScene;
uniform sampler2D uPrev;
uniform vec2 uRes;
uniform float uT;
uniform vec3 uColor;
uniform float uMode;

void main(){
  float t = clamp(uT, 0.0, 1.0);
  vec3 scn = texture2D(uScene, vUv).rgb;
  if (uMode < 0.5) {
    // ---- combine: push the accumulator outward, fade it, feed it ----
    vec2 d = vUv - 0.5;
    float pull = 0.006 + 0.040 * t;
    vec3 prev = texture2D(uPrev, 0.5 + d * (1.0 - pull)).rgb;
    // Decay, drifting the trail colour toward the tint (per-channel fade).
    float fade = 0.982 - 0.030 * t;
    prev *= fade * mix(vec3(1.0), uColor, 0.30 * t);
    // Feed: bright pixels, gated hard on saturation (neon streaks grab,
    // near-white credits text does not), attenuated in the central text
    // column and masked out of the centre so the elevator car + the roll
    // stay crisp.
    float maxc = max(scn.r, max(scn.g, scn.b));
    float minc = min(scn.r, min(scn.g, scn.b));
    float satw = smoothstep(0.45, 0.80, maxc - minc);
    float asp = uRes.x / max(uRes.y, 1.0);
    float rad = length(vec2(d.x * asp, d.y));
    float ring = smoothstep(0.10, 0.24, rad);
    float colw = mix(0.40, 1.0, smoothstep(0.42, 0.60, abs(d.x * asp)));
    vec3 feed = max(scn - 0.45, 0.0) * 1.8 * satw * ring * colw * t;
    gl_FragColor = vec4(max(prev, feed), 1.0);
  } else {
    // ---- present: scene + trails, screen-blended so nothing clips ----
    vec3 tr = texture2D(uPrev, vUv).rgb;
    vec3 c = 1.0 - (1.0 - scn) * (1.0 - tr);
    gl_FragColor = vec4(c, 1.0);
  }
}
`;

export const DRIVE_VS = `
attribute vec2 aPos;
void main(){
  gl_Position = vec4(aPos, 0.0, 1.0);
}
`;

export const DRIVE_FS = `
precision highp float;
uniform vec2 uSize;      // rect size, CSS px
uniform float uTexH;     // render-target height, texels (for the y flip)
uniform float uT;        // loop clock, seconds
uniform float uGlitch;   // tear intensity 0..1
uniform float uSplit;    // channel-split offset, CSS px (0 outside bursts)
uniform float uPx;       // art-pixel size, CSS px
uniform float uDim;      // darken the finished scene toward the menu black
uniform float uOffs[9];  // per-band tear offsets, CSS px
uniform float uSunSeed;  // sun-glitch hash seed, 0 = calm sun
uniform vec4 uPalmA[24]; // xb yb ht lean
uniform vec4 uPalmB[24]; // fogMix seed sway active
uniform vec4 uDebris[7]; // x y w h  (w <= 0 = unused slot)
uniform vec4 uDebrisC[7];// r g b a

float h11(float a, float b) {
  return fract(sin(a * 127.1 + b * 311.7) * 43758.5453);
}
float sdSeg(vec2 p, vec2 a, vec2 b) {
  vec2 pa = p - a, ba = b - a;
  float t = clamp(dot(pa, ba) / max(dot(ba, ba), 1e-6), 0.0, 1.0);
  return length(pa - ba * t);
}

// One pass of the scene at point p (rect-local CSS px). detail=false is the
// ghost-pass variant: skips the glow / stars / rain like the old renderer.
vec3 scene(vec2 p, bool detail) {
  float w = uSize.x, h = uSize.y;
  float horizon = h * 0.44;
  float ppu = w * 0.14;
  vec3 col;
  if (p.y < horizon) {
    // Sky: banded dusk gradient (22 bands).
    float f0 = floor(p.y / horizon * 22.0) / 22.0;
    col = f0 < 0.55
      ? mix(vec3(0.06, 0.02, 0.13), vec3(0.30, 0.06, 0.34), f0 / 0.55)
      : mix(vec3(0.30, 0.06, 0.34), vec3(0.86, 0.24, 0.33), (f0 - 0.55) / 0.45);
    float sr = h * 0.21;
    vec2 sc = vec2(w * 0.5, horizon - sr * 0.28);
    if (detail) {
      // Sun glow: two flat discs, like the old alpha circles.
      float d = length(p - sc);
      if (d < sr * 1.9) col = mix(col, vec3(1.0, 0.45, 0.35), 0.10);
      if (d < sr * 1.4) col = mix(col, vec3(1.0, 0.55, 0.35), 0.12);
      // Stars: one per sparse 42px cell, twinkling.
      vec2 cell = floor(p / 42.0);
      if (h11(cell.x + 11.0, cell.y + 17.0) < 0.15) {
        vec2 sp2 = (cell + vec2(h11(cell.x, cell.y * 7.31 + 1.0), h11(cell.x + 3.7, cell.y + 9.1))) * 42.0;
        if (sp2.y < horizon * 0.8) {
          float rad = 0.7 + h11(cell.x + 5.0, cell.y + 2.0) * 1.1;
          float tw = 0.5 + 0.5 * sin(uT * (0.8 + h11(cell.x + 8.0, cell.y + 4.0) * 2.2) + cell.x * 7.0 + cell.y * 13.0);
          if (length(p - sp2) < rad) col = mix(col, vec3(1.0, 0.95, 1.0), 0.5 * tw * (1.0 - sp2.y / horizon));
        }
      }
      // Digital rain: one 2px trail per column stride.
      float colW = w / 22.0;
      float ci = floor(p.x / colW);
      float cx = ci * colW + h11(ci, 31.0) * (colW - 2.0);
      if (p.x >= cx && p.x < cx + 2.0) {
        float spd = 26.0 + h11(ci, 32.0) * 70.0;
        float head = mod(uT * spd + h11(ci, 33.0) * 600.0, horizon + 40.0) - 20.0;
        float ra = 0.05 + 0.18 * uGlitch;
        for (int j = 0; j < 4; j++) {
          float yy = head - float(j) * 7.0;
          if (yy > 0.0 && yy < horizon && p.y >= yy && p.y < yy + 5.0)
            col = mix(col, vec3(0.35, 1.0, 0.65), ra * (1.0 - float(j) * 0.22));
        }
      }
    }
    // Sun: banded disc, cuts growing toward the bottom, glitch slide.
    float v = (p.y - (sc.y - sr)) / (2.0 * sr);
    if (v >= 0.0 && v < 1.0) {
      float si = floor(v * 26.0);
      float f0s = si / 26.0;
      float dy = abs((f0s + 0.5 / 26.0) * 2.0 * sr - sr);
      if (dy < sr) {
        float halfW = sqrt(sr * sr - dy * dy);
        float cut = f0s > 0.45 ? (f0s - 0.45) / 0.55 * 0.55 : 0.0;
        float dxs = uSunSeed > 0.5 ? (h11(uSunSeed, 400.0 + si) - 0.5) * 14.0 * uGlitch : 0.0;
        if (fract(v * 26.0) < 1.0 - cut && abs(p.x - sc.x - dxs) < halfW)
          col = mix(vec3(1.0, 0.88, 0.28), vec3(1.0, 0.22, 0.52), f0s);
      }
    }
  } else {
    // Ground + road, quantized in the same 48 screen rows as before.
    float row = floor((p.y - horizon) / (h - horizon) * 48.0);
    float ym = horizon + (h - horizon) * (row + 0.5) / 48.0;
    float z = min((h - horizon) / (ym - horizon), 400.0);
    float fog = pow(clamp(z / 36.0, 0.0, 1.0), 1.2);
    float pd = z + uT * 13.0;
    bool alt = mod(floor(pd / 2.4), 2.0) < 0.5;
    col = mix(alt ? vec3(0.060, 0.025, 0.100) : vec3(0.045, 0.015, 0.085),
              vec3(0.10, 0.035, 0.13), fog);
    float halfW = 3.0 * ppu / z;
    float ax = abs(p.x - w * 0.5);
    if (halfW > 1.0 && ax < halfW) {
      col = mix(alt ? vec3(0.130, 0.075, 0.190) : vec3(0.100, 0.055, 0.155),
                vec3(0.11, 0.045, 0.145), fog);
      float ew = max(halfW * 0.055, 1.2);
      if (ax > halfW - ew) {
        // Edge lines, alternating hot pink / pale.
        vec3 ec = mix(alt ? vec3(1.0, 0.32, 0.62) : vec3(0.95, 0.90, 0.95),
                      vec3(0.5, 0.2, 0.4), fog);
        col = mix(col, ec, clamp(1.0 - fog * 0.6, 0.0, 1.0));
      } else if (mod(floor(pd / 1.4), 2.0) < 0.5 && ax < max(halfW * 0.045, 1.0) * 0.5) {
        // Centre dashes rushing at the camera.
        col = mix(col, vec3(0.98, 0.92, 0.72), clamp(0.9 - fog * 0.7, 0.0, 1.0));
      }
    }
  }
  // Horizon glow line.
  if (abs(p.y - horizon) <= 1.0) col = mix(col, vec3(1.0, 0.42, 0.70), 0.9);
  // Palms, far to near (the uniform array is filled in draw order); a cheap
  // bounding test skips the segment math for nearly every pixel.
  for (int i = 0; i < 24; i++) {
    vec4 A = uPalmA[i];
    vec4 B = uPalmB[i];
    if (B.w < 0.5) continue;
    float ht = A.z;
    if (p.y > A.y + uPx || p.y < A.y - ht * 1.7 || abs(p.x - A.x) > ht * 1.4) continue;
    vec3 pc = mix(vec3(0.050, 0.015, 0.090), vec3(0.55, 0.16, 0.30), B.x * 0.8);
    bool hit = false;
    // Trunk: three tapering segments curving into the lean.
    vec2 p0 = A.xy;
    vec2 tp = p0;
    for (int s = 1; s <= 3; s++) {
      float f = float(s) / 3.0;
      vec2 p1 = vec2(A.x + A.w * ht * pow(f, 1.6), A.y - ht * f);
      if (sdSeg(p, p0, p1) < max(ht * 0.050 * (1.0 - 0.5 * f), uPx) * 0.5) hit = true;
      p0 = p1;
      tp = p1;
    }
    // Crown: drooping fronds fanned across the top.
    for (int k = 0; k < 7; k++) {
      float a = -3.14159265 * (0.12 + 0.76 * float(k) / 6.0) + B.z + (h11(B.y, 70.0 + float(k)) - 0.5) * 0.12;
      float len = ht * (0.38 + 0.10 * h11(B.y, 80.0 + float(k)));
      vec2 mid = tp + vec2(cos(a), sin(a)) * len * 0.6;
      float a2 = cos(a) >= 0.0 ? a + 0.7 : a - 0.7;
      vec2 e = mid + vec2(cos(a2), sin(a2)) * len * 0.5;
      float th = max(ht * 0.022, uPx);
      if (sdSeg(p, tp, mid) < th * 0.5 || sdSeg(p, mid, e) < max(th * 0.8, uPx) * 0.5) hit = true;
    }
    if (length(p - tp) < max(ht * 0.045, 1.5)) hit = true;
    if (hit) col = pc;
  }
  return col;
}

void main() {
  // This pass runs at ART RESOLUTION (one fragment per art pixel; the
  // result is upscaled NEAREST by a textured quad), so each fragment IS
  // its cell centre — the quantization comes for free and the whole scene
  // costs ~1/(px*dpr)^2 of a native-resolution evaluation.
  vec2 p = vec2(gl_FragCoord.x, uTexH - gl_FragCoord.y) * uPx;
  // Tear: this band samples the scene shifted sideways; where the slice
  // moved away, the backing void shows through.
  float bandH = uSize.y / 9.0;
  float band = clamp(floor(p.y / bandH), 0.0, 8.0);
  float dx = 0.0;
  for (int i = 0; i < 9; i++) if (float(i) == band) dx = uOffs[i];
  p.x -= dx;
  vec3 col;
  if (p.x < -0.5 * uPx || p.x >= uSize.x + 0.5 * uPx) {
    col = vec3(0.01, 0.0, 0.03);
  } else {
    col = scene(p, true);
    // Channel split: red/cyan ghost passes over the base, like the old
    // translated re-draws (0.75px threshold mirrored from scene_passes).
    if (abs(uSplit) >= 0.75) {
      col = mix(col, scene(p + vec2(uSplit, 0.0), false) * vec3(1.0, 0.12, 0.25), 0.26);
      col = mix(col, scene(p - vec2(uSplit, 0.0), false) * vec3(0.10, 0.90, 1.0), 0.26);
    }
    // Neon debris blocks flash on top.
    for (int i = 0; i < 7; i++) {
      vec4 r = uDebris[i];
      if (r.z <= 0.0) continue;
      if (p.x >= r.x && p.x < r.x + r.z && p.y >= r.y && p.y < r.y + r.w)
        col = mix(col, uDebrisC[i].rgb, uDebrisC[i].a);
    }
  }
  // The menu dim, folded in here: what used to be a full-screen alpha rect
  // blended over the backdrop is a free mix at art resolution.
  col = mix(col, vec3(0.02, 0.01, 0.04), uDim);
  gl_FragColor = vec4(col, 1.0);
}
`;

export const BACKDROP_FS = `
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
uniform vec2 uSize;   // rect size, CSS px
uniform float uTexH;  // render-target height, texels (for the y flip)
uniform float uT;     // clock, seconds
uniform float uPx;    // art-pixel size, CSS px

void main() {
  vec2 p = vec2(gl_FragCoord.x, uTexH - gl_FragCoord.y) * uPx;
  // Aspect-preserving field coordinate (waves keep their shape on resize).
  vec2 q = p / max(uSize.y, 1.0);
  // Three slow interference fields (periods ~15-45 s).
  float a = sin(q.x * 4.1 + uT * 0.23) + sin(q.y * 3.3 - uT * 0.17);
  float b = sin((q.x + q.y) * 2.6 - uT * 0.13) + sin(q.x * 1.7 - q.y * 2.9 + uT * 0.19);
  float c = sin(q.x * 5.3 - uT * 0.11) * sin(q.y * 4.7 + uT * 0.29);
  // Near-black void base + heavily-darkened neon crests (peak channel stays
  // ~0.07 — below ASPHALT_DARK, the darkest floor base in src/palette.rs).
  vec3 col = vec3(0.008, 0.005, 0.014);
  col += vec3(0.042, 0.005, 0.026) * smoothstep(0.9, 1.9, a);   // hot pink
  col += vec3(0.005, 0.032, 0.040) * smoothstep(0.9, 1.9, b);   // cyan
  col += vec3(0.016, 0.007, 0.028) * (0.5 + 0.5 * c);           // violet wash
  gl_FragColor = vec4(col, 1.0);
}
`;
