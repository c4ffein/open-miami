"use strict";
/* =========================================================================
   OPEN MIAMI - reusable 3D -> stylized 2D sprite renderer.
   Vanilla WebGL 1, no libraries. Shared by renderer.js (the game),
   tools/inspector.html and shoggoth-core.js (the boss, which builds on the
   same two-pass pipeline).

   Exports:
     PALETTES                       - color name -> {body,accent,trim}
     POSES                          - list of pose names
     WEAPONS                        - list of weapon names (fist/pistol/machinegun/shotgun)
     WEAPON_MODELS                  - name -> array of box parts (the 3D weapon models)
     GROUND_WEAPON_MODELS           - [bar, pistol, machinegun, shotgun] box models
                                      as they lie on the ground (pickups / thrown);
                                      RobotPipeline.renderGun draws one flat,
                                      top-down, spun to a resting angle
     M4                             - the pooled column-major mat4 helpers
     orbitVP / topDownVP            - camera builders (view-projection matrices)
     SpritePipeline                 - the pass-1 target + post pass base class that
                                      RobotPipeline (here) and ShoggothPipeline
                                      (shoggoth-core.js) extend: it owns the shared
                                      "inked" post-process, the FBO and the compile
                                      helpers, so no scene duplicates them
     createRobotPipeline(gl, {rt})  - the two-pass pipeline on an EXISTING WebGL
                                      context: .render(opts, target) draws one
                                      robot into a caller-provided framebuffer
                                      rect (or the canvas). This is what the game
                                      renderer uses to draw robots live, every
                                      frame, inside its own GL context.
     CanvasRenderer / makeBaker     - a pipeline bound to a canvas of its own /
                                      a "one frame -> fresh canvas" baker factory
                                      (both generic: shoggoth-core.js reuses them)
     createRenderer(canvas)         - a robot CanvasRenderer (owns a context +
                                      a pipeline); .render({...,weapon}) draws
                                      the held weapon
     bakeSprite({pose,color,facingDeg,px,time,size,weapon}) -> HTMLCanvasElement
                                      renders ONE baked top-down sprite frame.

   The robot is built from boxes + a tiny skeleton. A two-pass pipeline:
     pass 1: lit boxes -> offscreen RGBA texture (alpha carries a part id)
     pass 2: edge-detect + posterize + pixelate -> target ("inked" look)
   ========================================================================= */

/* ---------- palettes (player + 3 rogue palettes) ---------- */
export const PALETTES = {
  // body, accent(limbs/visor glow), dark trim
  coral:   {body:[0.98,0.52,0.42], accent:[1.0,0.75,0.55], trim:[0.35,0.14,0.12]},
  red:     {body:[0.86,0.16,0.18], accent:[1.0,0.45,0.30], trim:[0.28,0.05,0.06]},
  magenta: {body:[0.86,0.18,0.72], accent:[1.0,0.45,0.95], trim:[0.30,0.06,0.24]},
  violet:  {body:[0.55,0.35,0.90], accent:[0.75,0.60,1.0], trim:[0.18,0.12,0.34]},
};

export const POSES = ["idle", "walk", "shoot", "hit", "downed",
                      "downed_headless", "kick", "stomp"];

/* ---------- weapons ----------
   Box-built weapon models in the same style as the robot. Each model is a small
   list of boxes expressed in the GUN-HAND's local frame (the right forearm's
   "elbow" node), pre-anchored at the grip. In that frame -Y points down the arm,
   which — once the arm is rotated forward to aim — becomes world +Z (forward) and
   slightly down: exactly the direction the shoot-pose barrel already sticks out.
   So a weapon whose body extends toward -Y reads as "held out in front" from the
   straight-down top-down bake.

   Each box: {t:[x,y,z], s:[x,y,z], c:[r,g,b] | "accent", id}
     - "accent" pulls the palette accent (muzzle/detail glow) so weapons tint
       with the character; solid arrays are fixed gunmetal.
   "fist" is the bare-hand / no-weapon case: no boxes, arm stays in its pose. */
export const WEAPONS = ["fist", "pistol", "machinegun", "shotgun"];

// Gun palette — four fixed tones picked to SURVIVE the ground bake's color
// path (mix 0.28 toward GROUND_ACCENT for up-facing normals, then 4-level
// posterize): each lands on a distinct quantized tone from straight above,
// so slide/frame/grip/wood separation stays readable on the tiny ground art
// even with interior ink disabled. Metallic + muted, per the art direction.
const GUN_METAL = [0.44, 0.46, 0.52]; // receiver / frame steel  (-> 0.50 grey)
const GUN_LIGHT = [0.62, 0.65, 0.72]; // slide top / bright steel (-> 0.75 grey)
const GUN_DARK  = [0.13, 0.14, 0.17]; // grip / mag / barrel darks (-> 0.25 grey)
const GUN_WOOD  = [0.42, 0.20, 0.12]; // shotgun furniture       (-> warm brown)

/* Ground models: the weapons as they lie on the floor (pickups / thrown
   weapons), indexed to match the Rust WeaponType mapping used by the
   GUNPICKUP opcode: 0 bar (melee), 1 pistol, 2 machinegun, 3 shotgun.
   The three guns reuse the held models; the BAR (the melee weapon, which is
   never rendered in-hand) gets its own box model. `GROUND_WEAPON_CENTER` is
   the local +Y shift that centres each model on its midpoint so a resting /
   spinning weapon rotates around its own centre. */
const BAR_MODEL = [
  {t:[0,-0.02, 0.0], s:[0.11,1.10,0.11], c:GUN_METAL, id:0.90}, // the bar itself
  {t:[0, 0.40, 0.0], s:[0.16,0.26,0.16], c:GUN_DARK,  id:0.88}, // taped grip wrap
  {t:[0, 0.25, 0.0], s:[0.14,0.05,0.14], c:GUN_LIGHT, id:0.87}, // wrap band (grip edge)
  {t:[0, 0.55, 0.0], s:[0.14,0.06,0.14], c:GUN_LIGHT, id:0.87}, // pommel end cap
  {t:[0,-0.545,0.0], s:[0.15,0.10,0.15], c:"accent",  id:0.98}, // scuffed strike tip
];

/* Model frame conventions (gun-hand local): -Y = forward (muzzle), +Y = rear
   (grip / stock, up the arm), +Z = the gun's top (sights), -Z = down (grip,
   magazine), X = thickness. On the ground the model lies on its side, so the
   Y x Z side profile faces the camera and boxes with a LARGER X extent win
   the depth test over the base parts they overlap (serrations, ports, pump). */
export const WEAPON_MODELS = {
  fist: [],
  pistol: [
    {t:[0,-0.05,  0.02 ], s:[0.15, 0.40, 0.08 ], c:GUN_METAL, id:0.90}, // frame
    {t:[0,-0.09,  0.09 ], s:[0.16, 0.44, 0.085], c:GUN_LIGHT, id:0.91}, // slide (rides on top)
    {t:[0, 0.085, 0.095], s:[0.175,0.07, 0.08 ], c:GUN_DARK,  id:0.89}, // rear slide serrations
    {t:[0,-0.04,  0.105], s:[0.175,0.10, 0.05 ], c:GUN_DARK,  id:0.89}, // ejection port
    {t:[0,-0.27,  0.145], s:[0.18, 0.04, 0.04 ], c:GUN_DARK,  id:0.88}, // front sight nub
    {t:[0,-0.35,  0.09 ], s:[0.18, 0.07, 0.06 ], c:"accent",  id:0.98}, // muzzle (protrudes past the slide)
    {t:[0, 0.13, -0.07 ], s:[0.13, 0.12, 0.12 ], c:GUN_DARK,  id:0.88}, // grip upper
    {t:[0, 0.175,-0.165], s:[0.13, 0.12, 0.10 ], c:GUN_DARK,  id:0.88}, // grip lower (raked back)
    {t:[0, 0.20, -0.235], s:[0.145,0.12, 0.04 ], c:GUN_LIGHT, id:0.87}, // grip base plate
    {t:[0,-0.10, -0.095], s:[0.045,0.03, 0.15 ], c:GUN_DARK,  id:0.86}, // trigger guard front
    {t:[0,-0.02, -0.16 ], s:[0.045,0.19, 0.03 ], c:GUN_DARK,  id:0.86}, // trigger guard bottom
  ],
  machinegun: [
    {t:[0, 0.03,  0.02 ], s:[0.16, 0.44, 0.13 ], c:GUN_METAL, id:0.90}, // receiver
    {t:[0,-0.34,  0.02 ], s:[0.14, 0.30, 0.10 ], c:GUN_DARK,  id:0.89}, // handguard (darker, slimmer)
    {t:[0,-0.55,  0.035], s:[0.065,0.14, 0.055], c:GUN_DARK,  id:0.88}, // protruding barrel
    {t:[0,-0.645, 0.035], s:[0.085,0.06, 0.075], c:"accent",  id:0.98}, // muzzle device
    {t:[0,-0.005,-0.13 ], s:[0.11, 0.13, 0.11 ], c:GUN_DARK,  id:0.87}, // magazine (at the well)
    {t:[0,-0.035,-0.215], s:[0.11, 0.12, 0.09 ], c:GUN_DARK,  id:0.87}, // magazine (tips forward)
    {t:[0,-0.075,-0.285], s:[0.11, 0.11, 0.07 ], c:GUN_DARK,  id:0.87}, // magazine toe (the curve)
    {t:[0, 0.185,-0.135], s:[0.10, 0.10, 0.13 ], c:GUN_DARK,  id:0.88}, // pistol grip
    {t:[0, 0.355,-0.05 ], s:[0.11, 0.25, 0.10 ], c:GUN_METAL, id:0.89}, // stock (dropped a touch)
    {t:[0, 0.505,-0.05 ], s:[0.13, 0.05, 0.14 ], c:GUN_DARK,  id:0.88}, // butt pad
    {t:[0,-0.455, 0.115], s:[0.05, 0.035,0.075], c:GUN_DARK,  id:0.86}, // front sight post
    {t:[0, 0.145, 0.105], s:[0.06, 0.05, 0.045], c:GUN_DARK,  id:0.86}, // rear sight
  ],
  shotgun: [
    {t:[0, 0.10,  0.03 ], s:[0.16, 0.30, 0.15 ], c:GUN_METAL, id:0.90}, // receiver
    {t:[0,-0.32,  0.10 ], s:[0.12, 0.58, 0.07 ], c:GUN_METAL, id:0.89}, // long barrel (on top; steel so
    //   the held top-down view keeps a bright stick, like the legacy model)
    {t:[0,-0.28, -0.01 ], s:[0.07, 0.46, 0.05 ], c:GUN_DARK,  id:0.88}, // magazine tube under it
    {t:[0,-0.30, -0.02 ], s:[0.17, 0.18, 0.10 ], c:GUN_WOOD,  id:0.87}, // pump handle
    {t:[0, 0.33, -0.06 ], s:[0.13, 0.26, 0.13 ], c:GUN_WOOD,  id:0.88}, // shoulder stock (dropped)
    {t:[0, 0.475,-0.065], s:[0.145,0.05, 0.13 ], c:GUN_DARK,  id:0.87}, // butt pad
    {t:[0,-0.575, 0.155], s:[0.05, 0.03, 0.04 ], c:GUN_LIGHT, id:0.86}, // bead sight
    {t:[0,-0.645, 0.10 ], s:[0.10, 0.05, 0.08 ], c:"accent",  id:0.98}, // muzzle glow
  ],
};

export const GROUND_WEAPON_MODELS = [
  BAR_MODEL,               // 0 = melee bar
  WEAPON_MODELS.pistol,    // 1
  WEAPON_MODELS.machinegun,// 2
  WEAPON_MODELS.shotgun,   // 3
];
// Local +Y shift that centres each ground model (the guns' mass sits toward
// the muzzle in the gun-hand frame).
const GROUND_WEAPON_CENTER = [0.01, 0.06, 0.07, 0.085];
// Muzzle-glow accent used for weapons on the ground (no owner to tint them).
const GROUND_ACCENT = [1.0, 0.75, 0.55];

/* ---------- tiny mat4 math (column-major, like GL) ----------
   Every matrix comes out of a bump-allocated scratch pool that is reset at the
   start of each render(): a robot is ~300 short-lived matrices, and recycling
   them keeps the per-frame path allocation-free once the pool has warmed up.
   Matrices are only valid until the next M4.reset() — nothing holds them. */
const _m4pool = [];
let _m4used = 0;
function m4alloc(){
  if(_m4used < _m4pool.length) return _m4pool[_m4used++];
  const m = new Float32Array(16);
  _m4pool.push(m); _m4used++;
  return m;
}
const _n3scratch = new Float32Array(9);
export const M4 = {
  reset(){ _m4used = 0; },
  ident(){const m=m4alloc();m.fill(0);m[0]=m[5]=m[10]=m[15]=1;return m;},
  mul(a,b){ // a*b
    const o=m4alloc();
    for(let r=0;r<4;r++)for(let c=0;c<4;c++){
      o[c*4+r]=a[0*4+r]*b[c*4+0]+a[1*4+r]*b[c*4+1]+a[2*4+r]*b[c*4+2]+a[3*4+r]*b[c*4+3];
    }
    return o;
  },
  translate(x,y,z){const m=M4.ident();m[12]=x;m[13]=y;m[14]=z;return m;},
  scale(x,y,z){const m=M4.ident();m[0]=x;m[5]=y;m[10]=z;return m;},
  rotX(a){const c=Math.cos(a),s=Math.sin(a);const m=M4.ident();m[5]=c;m[6]=s;m[9]=-s;m[10]=c;return m;},
  rotY(a){const c=Math.cos(a),s=Math.sin(a);const m=M4.ident();m[0]=c;m[2]=-s;m[8]=s;m[10]=c;return m;},
  rotZ(a){const c=Math.cos(a),s=Math.sin(a);const m=M4.ident();m[0]=c;m[1]=s;m[4]=-s;m[5]=c;return m;},
  ortho(l,r,b,t,n,f){
    const m=M4.ident();
    m[0]=2/(r-l);m[5]=2/(t-b);m[10]=-2/(f-n);
    m[12]=-(r+l)/(r-l);m[13]=-(t+b)/(t-b);m[14]=-(f+n)/(f-n);
    return m;
  },
  // look from eye toward center, up. returns view matrix.
  lookAt(eye,center,up){
    const z=norm(sub(eye,center));
    const x=norm(cross(up,z));
    const y=cross(z,x);
    const m=M4.ident();
    m[0]=x[0];m[4]=x[1];m[8]=x[2];
    m[1]=y[0];m[5]=y[1];m[9]=y[2];
    m[2]=z[0];m[6]=z[1];m[10]=z[2];
    m[12]=-dot(x,eye);m[13]=-dot(y,eye);m[14]=-dot(z,eye);
    return m;
  },
  // 3x3 normal matrix (upper-left 3x3; fine for our rotations + mild scales).
  // Returns a shared scratch: consume it (uniformMatrix3fv) before the next call.
  normalFromModel(m){
    const o=_n3scratch;
    o[0]=m[0];o[1]=m[1];o[2]=m[2]; o[3]=m[4];o[4]=m[5];o[5]=m[6]; o[6]=m[8];o[7]=m[9];o[8]=m[10];
    return o;
  }
};
function sub(a,b){return [a[0]-b[0],a[1]-b[1],a[2]-b[2]];}
function cross(a,b){return [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]];}
function dot(a,b){return a[0]*b[0]+a[1]*b[1]+a[2]*b[2];}
function norm(a){const l=Math.hypot(a[0],a[1],a[2])||1;return [a[0]/l,a[1]/l,a[2]/l];}

/* ---------- shaders ----------
   The robot scene pass is MATRIX-PALETTE SKINNED: the whole robot (and the
   held weapon / a ground weapon) lives in ONE static merged vertex buffer, a
   per-vertex part index selects the part's pose matrix out of a mat4 uniform
   array, and per-vertex selectors pick the body/accent colors out of a small
   color table — so a robot is ONE drawArrays (two when armed: body + weapon)
   instead of one call per box. Uniform budget: 27 mat4 (108 vec4) + uVP (4) +
   8 vec3 colors (8) = 120 vec4, inside the WebGL1 minimum of 128.
   Lighting / part-id-in-alpha are EXACTLY the legacy per-part shader: the
   normal is transformed by the pose matrix's upper-left 3x3 (what
   M4.normalFromModel produced) and the color math is unchanged, the values
   just arrive through varyings (constant across a part) instead of uniforms. */
const PALETTE_PARTS = 27; // 15 body slots (incl. the bare-hand barrel) + 12 weapon boxes
const WEAPON_SLOT0 = 15; // palette slot of a held weapon's first box
const sceneVS = `
attribute vec3 aPos;
attribute vec3 aNormal;
attribute vec4 aExtra;  // x part/palette index, y color sel, z accent sel, w part id
uniform mat4 uVP;
uniform mat4 uPart[${PALETTE_PARTS}];
uniform vec3 uColors[8];
varying vec3 vN;
varying vec3 vColor;
varying vec3 vAccent;
varying float vId;
void main(){
  mat4 M = uPart[int(aExtra.x + 0.5)];
  gl_Position = uVP * (M * vec4(aPos,1.0));
  // upper-left 3x3 of the pose matrix = the legacy uNormalMat
  vN = normalize(mat3(M[0].xyz, M[1].xyz, M[2].xyz) * aNormal);
  vColor = uColors[int(aExtra.y + 0.5)];
  vAccent = uColors[int(aExtra.z + 0.5)];
  vId = aExtra.w;
}
`;
const sceneFS = `
precision mediump float;
varying vec3 vN;
varying vec3 vColor;
varying vec3 vAccent;
varying float vId;
void main(){
  vec3 L = normalize(vec3(0.35, 0.9, 0.45));
  float ndl = max(dot(normalize(vN), L), 0.0);
  float amb = 0.35;
  float shade = amb + ndl*0.75;
  vec3 base = mix(vColor, vAccent, clamp(vN.y*0.5+0.2,0.0,1.0)*0.4);
  vec3 col = base * shade;
  // store part id in alpha so post pass can detect part boundaries as edges
  gl_FragColor = vec4(col, vId);
}
`;
const postVS = `
attribute vec2 aPos;
varying vec2 vUv;
void main(){ vUv = aPos*0.5+0.5; gl_Position = vec4(aPos,0.0,1.0); }
`;
const postFS = `
precision mediump float;
varying vec2 vUv;
uniform sampler2D uTex;
uniform vec2 uTexel;   // 1/size
uniform float uPx;     // pixel block size in px
uniform vec2 uSize;    // texture size
uniform float uTransparent; // 1.0 -> background blocks output alpha 0
uniform float uEdge;   // luma-gradient threshold that inks an edge
uniform float uAiInk;  // 1.0 = ink part-id boundaries (0 for tiny art, where
                       //  every texel borders one and would come out black)
float luma(vec3 c){ return dot(c, vec3(0.299,0.587,0.114)); }
vec4 samp(vec2 uv){ return texture2D(uTex, uv); }
void main(){
  vec2 px = uSize;
  vec2 block = floor(vUv*px/uPx)*uPx + uPx*0.5;
  vec2 uv = block/px;

  vec4 c = samp(uv);
  vec3 col = c.rgb;

  vec2 t = uTexel*uPx;
  float l00=luma(samp(uv+vec2(-t.x,-t.y)).rgb);
  float l10=luma(samp(uv+vec2( 0.0,-t.y)).rgb);
  float l20=luma(samp(uv+vec2( t.x,-t.y)).rgb);
  float l01=luma(samp(uv+vec2(-t.x, 0.0)).rgb);
  float l21=luma(samp(uv+vec2( t.x, 0.0)).rgb);
  float l02=luma(samp(uv+vec2(-t.x, t.y)).rgb);
  float l12=luma(samp(uv+vec2( 0.0, t.y)).rgb);
  float l22=luma(samp(uv+vec2( t.x, t.y)).rgb);
  float gx = -l00 -2.0*l01 -l02 + l20 + 2.0*l21 + l22;
  float gy = -l00 -2.0*l10 -l20 + l02 + 2.0*l12 + l22;
  float lumEdge = sqrt(gx*gx+gy*gy);

  float a = c.a;
  float ai = max(max(abs(a-samp(uv+vec2(t.x,0.0)).a),abs(a-samp(uv+vec2(-t.x,0.0)).a)),
                 max(abs(a-samp(uv+vec2(0.0,t.y)).a),abs(a-samp(uv+vec2(0.0,-t.y)).a)));

  float silh = 0.0;
  if(a < 0.02){
    float near = max(max(samp(uv+vec2(t.x,0.0)).a,samp(uv+vec2(-t.x,0.0)).a),
                     max(samp(uv+vec2(0.0,t.y)).a,samp(uv+vec2(0.0,-t.y)).a));
    silh = near>0.02 ? 1.0 : 0.0;
  }

  float edge = max(max(step(uEdge, lumEdge), step(0.03, ai) * uAiInk), silh);

  float levels = 4.0;
  col = floor(col*levels + 0.5)/levels;

  if(a < 0.02 && silh < 0.5){
    if(uTransparent > 0.5){ gl_FragColor = vec4(0.0); return; }
    col = vec3(0.055,0.07,0.09);
  }

  col = mix(col, vec3(0.02,0.02,0.03), edge);

  gl_FragColor = vec4(col,1.0);
}
`;

/* ---------- unit cube geometry (positions + normals), centered, size 1 ---------- */
function makeCube(){
  const p=[], n=[];
  const faces=[
    {n:[0,0,1],  v:[[-.5,-.5,.5],[.5,-.5,.5],[.5,.5,.5],[-.5,.5,.5]]},
    {n:[0,0,-1], v:[[.5,-.5,-.5],[-.5,-.5,-.5],[-.5,.5,-.5],[.5,.5,-.5]]},
    {n:[1,0,0],  v:[[.5,-.5,.5],[.5,-.5,-.5],[.5,.5,-.5],[.5,.5,.5]]},
    {n:[-1,0,0], v:[[-.5,-.5,-.5],[-.5,-.5,.5],[-.5,.5,.5],[-.5,.5,-.5]]},
    {n:[0,1,0],  v:[[-.5,.5,.5],[.5,.5,.5],[.5,.5,-.5],[-.5,.5,-.5]]},
    {n:[0,-1,0], v:[[-.5,-.5,-.5],[.5,-.5,-.5],[.5,-.5,.5],[-.5,-.5,.5]]},
  ];
  for(const f of faces){
    const [a,b,c,d]=f.v;
    for(const tri of [[a,b,c],[a,c,d]]) for(const vtx of tri){ p.push(...vtx); n.push(...f.n); }
  }
  return {pos:new Float32Array(p), nrm:new Float32Array(n), count:p.length/3};
}

/* ---------- the merged skinned mesh (built once per pipeline) ----------
   Every box any robot render can need, as instances of the unit cube tagged
   with (palette index, color selector, accent selector, part id) — 10 floats
   per vertex, interleaved. Layout (in the LEGACY DRAW ORDER, so depth-equal
   ties resolve identically):
     [0]   14 body cubes  (torso, head, visor, hips, legs, arms)
     [504] the bare-hand barrel cube (drawn by extending the body range)
     then the held weapon models (palette slots 15+), then the ground weapon
     models (palette slots 0+; their own color/id tags — see renderGun).
   Color selectors index the per-render uColors table:
     0 pal.body  1 pal.accent  2 pal.trim  3 GUN_METAL  4 GUN_DARK
     5 GROUND_ACCENT  6 GUN_LIGHT  7 GUN_WOOD */
const COL_BODY=0, COL_ACCENT=1, COL_TRIM=2, COL_METAL=3, COL_DARK=4, COL_GROUND=5,
      COL_LIGHT=6, COL_WOOD=7;
// (colorSel, accentSel, id) per body palette slot — the exact colors/ids the
// legacy _drawPart calls passed (ids: 0.4/0.45/0.5 +- 0.32 legs, 0.6/0.65
// +- 0.62 arms; the clamping of -0.02 / 1.2x to the 8-bit alpha range is
// unchanged, it happens at framebuffer write time exactly as before).
const BODY_PART_TAGS = [
  [COL_BODY,   COL_ACCENT, 0.2],        // 0  torso
  [COL_BODY,   COL_ACCENT, 0.35],       // 1  head
  [COL_ACCENT, COL_ACCENT, 0.9],        // 2  visor strip
  [COL_TRIM,   COL_BODY,   0.25],       // 3  hips
  [COL_BODY,   COL_ACCENT, 0.4 -0.32],  // 4  L thigh
  [COL_TRIM,   COL_ACCENT, 0.45-0.32],  // 5  L shin
  [COL_TRIM,   COL_BODY,   0.5 -0.32],  // 6  L foot
  [COL_BODY,   COL_ACCENT, 0.4 +0.32],  // 7  R thigh
  [COL_TRIM,   COL_ACCENT, 0.45+0.32],  // 8  R shin
  [COL_TRIM,   COL_BODY,   0.5 +0.32],  // 9  R foot
  [COL_BODY,   COL_ACCENT, 0.6 -0.62],  // 10 L upper arm
  [COL_TRIM,   COL_ACCENT, 0.65-0.62],  // 11 L forearm
  [COL_BODY,   COL_ACCENT, 0.6 +0.62],  // 12 R upper arm
  [COL_TRIM,   COL_ACCENT, 0.65+0.62],  // 13 R forearm
  [COL_ACCENT, COL_ACCENT, 0.95],       // 14 bare-hand barrel
];
const BODY_CUBES = 14; // without the barrel; 15 with
function buildMergedMesh(){
  const cube = makeCube();
  const data = [];
  let cubes = 0;
  function addCube(partIdx, colSel, accSel, id){
    for(let v=0; v<cube.count; v++){
      data.push(cube.pos[v*3], cube.pos[v*3+1], cube.pos[v*3+2],
                cube.nrm[v*3], cube.nrm[v*3+1], cube.nrm[v*3+2],
                partIdx, colSel, accSel, id);
    }
    cubes++;
  }
  const weaponColSel = (c, accentSel) =>
    c === "accent" ? accentSel :
    c === GUN_METAL ? COL_METAL :
    c === GUN_LIGHT ? COL_LIGHT :
    c === GUN_WOOD  ? COL_WOOD  : COL_DARK;
  BODY_PART_TAGS.forEach(([colSel, accSel, id], slot) => addCube(slot, colSel, accSel, id));
  const held = {};
  for(const w of ["pistol", "machinegun", "shotgun"]){
    const first = cubes * cube.count;
    // One shared id (0.9) for the whole weapon, like the ground bake: the
    // detailed models are a dozen small boxes, and per-box ids would ink
    // every texel of the tiny held art black. The weapon still outlines
    // against the arm/body (different ids); interior read = color separation.
    WEAPON_MODELS[w].forEach((b, j) =>
      addCube(WEAPON_SLOT0 + j, weaponColSel(b.c, COL_ACCENT), COL_ACCENT, 0.9));
    held[w] = {first, count: cubes*cube.count - first};
  }
  const ground = GROUND_WEAPON_MODELS.map((model) => {
    const first = cubes * cube.count;
    // one shared id (0.9) + the ground accent for every box: see renderGun
    model.forEach((b, j) => addCube(j, weaponColSel(b.c, COL_GROUND), COL_GROUND, 0.9));
    return {first, count: cubes*cube.count - first};
  });
  return {data: new Float32Array(data), vertCount: cube.count, held, ground};
}

/* ---------- camera builders ---------- */
// TRUE straight-down top-down — the eye is directly over the character (no tilt),
// so a facing rotation is just the same sprite spun in-plane (identical from every
// direction, one bake per pose). This is exactly what the in-game camera sees.
export function topDownVP(halfV){
  halfV = halfV || 2.05;
  const proj = M4.ortho(-halfV,halfV,-halfV,halfV,0.1,40);
  const eye=[0, 9, 0], center=[0,0.9,0], up=[0,0,-1];
  return M4.mul(proj, M4.lookAt(eye,center,up));
}
// free orbit: yaw + pitch around the character, ortho so scale is stable.
export function orbitVP(yaw, pitch, halfV, center){
  halfV = halfV || 2.35;
  center = center || [0,0.95,0];
  const dist=12;
  pitch = Math.max(-1.45, Math.min(1.45, pitch));
  const cp=Math.cos(pitch), sp=Math.sin(pitch);
  const cy=Math.cos(yaw),   sy=Math.sin(yaw);
  const eye=[center[0]+dist*cp*sy, center[1]+dist*sp, center[2]+dist*cp*cy];
  // keep up stable; near-vertical is clamped above so lookAt won't degenerate.
  const up=[0,1,0];
  const proj = M4.ortho(-halfV,halfV,-halfV,halfV,0.1,60);
  return M4.mul(proj, M4.lookAt(eye,center,up));
}

/* ---------- pose -> skeleton drive ---------- */
// Returns the per-frame joint angles / offsets for a pose at a given time.
// `relaxed` (no weapon held) softens idle/walk into an off-duty stance: arms
// hanging loose at the sides, slightly splayed out from the hips with a soft
// elbow bend, and an easy walk swing. Combat/impact poses ignore it.
function posePlan(pose, time, relaxed){
  const walkPhase = time*2.0*Math.PI;
  const swing  = Math.sin(walkPhase)*0.6;
  const swing2 = Math.sin(walkPhase+Math.PI)*0.6;
  const ss = (v)=>{ v = Math.min(Math.max(v, 0), 1); return v*v*(3-2*v); };

  // defaults (a neutral standing rig)
  const P = {
    bob:0, lean:0, zback:0,
    legA:0, legB:0,
    armLp:0.05, armRp:0.05, shoot:false,
    armRaise:0,          // extra shoulder-raise for both arms (defensive/idle)
    armOut:0,            // sideways splay of both arms (relaxed hang)
    elbow:0,             // forearm bend at the elbow (relaxed hang)
    recoil:0,
  };

  switch(pose){
    case "walk":
      P.bob = Math.abs(Math.sin(walkPhase))*0.08;
      P.legA = swing;  P.legB = swing2;
      P.armLp = swing2; P.armRp = swing;   // arms counter-swing to legs
      if(relaxed){
        // natural unarmed walk: an easy half swing, arms loose at the sides
        P.armLp = swing2*0.55; P.armRp = swing*0.55;
        P.armOut = 0.10; P.elbow = 0.28;
      }
      break;

    case "shoot":
      P.shoot = true;
      P.legA = 0.12; P.legB = -0.12;
      P.armLp = 0.5;                        // support arm braces forward-ish
      P.recoil = Math.max(0.0, Math.sin(time*10.0))*0.18;
      break;

    case "idle": {
      const breath = Math.sin(time*1.9);
      P.bob   = breath*0.045;               // gentle chest/torso bob
      P.legA  = 0.015; P.legB = -0.015;     // weight shift, feet planted
      P.armLp = 0.08 + breath*0.05;         // arms sway slightly out of phase
      P.armRp = 0.08 - breath*0.05;
      if(relaxed){
        // at ease: arms hang straight down the sides, breathing gently
        P.armLp = 0.02 + breath*0.03;
        P.armRp = 0.02 - breath*0.03;
        P.armOut = 0.14 + breath*0.02;
        P.elbow = 0.14;
      }
      break;
    }

    case "hit": {
      // periodic flinch: a sharp recoil back that decays, then repeats.
      const period = 1.3;
      const p = (time % period) / period;   // 0..1
      const env = Math.exp(-p*7.0);         // spike at impact, quick decay
      P.lean  = 0.55*env;                   // whole body rocks backward
      P.zback = -0.28*env;                  // and shoves back off its feet
      P.bob   = -0.05*env;
      P.legA  = -0.25*env; P.legB = 0.18*env;
      P.armRaise = 0.9*env;                 // arms fling up defensively
      P.armLp = 0.2; P.armRp = 0.2;
      break;
    }

    case "downed_headless": // a KICK victim: same sprawl, head cubes skipped
    case "downed": {
      // knocked flat on its back, limbs askew. `time` is seconds since the
      // knockdown landed: the first ~0.25s eases from upright to sprawled
      // (the fall), with a decaying landing wobble, then the body lies still.
      // The game sets the facing so the body topples AWAY from the blow
      // (the lean rotates it backward, i.e. opposite the sprite's facing).
      const k = Math.min(time/0.25, 1);
      const e = k*k*(3-2*k);                // smoothstep fall
      const s = time > 0.25
        ? Math.sin((time-0.25)*9.0)*Math.exp(-(time-0.25)*3.0) : 0;
      P.lean  = 1.42*e + 0.10*s;            // topple flat onto its back
      P.zback = 0.35*e;                     // slide with the blow's momentum
      P.bob   = -0.06*e;
      P.legA  = 0.55*e; P.legB = -0.38*e;   // legs splayed
      P.armLp = -0.45*e; P.armRp = 0.35*e;  // arms askew...
      P.armRaise = 1.25*e;                  // ...flung up past the head
      P.headless = (pose === "downed_headless");
      break;
    }

    case "kick": {
      // The head-kick finisher. `time` is seconds into the finisher (the
      // impact lands at ~0.28s, see FinisherKind::Kick): the kicking leg
      // cocks back, sweeps through horizontally at the impact, then eases
      // back down while the body leans back off the kick for balance.
      const t = Math.max(time, 0);
      const wind   = ss(t/0.16);            // cock the leg back...
      const sweep  = ss((t-0.16)/0.12);     // ...sweep it clean through
      const settle = ss((t-0.36)/0.19);     // ...and put it back down
      const k = 1 - settle*0.85;
      P.legA  = 0.14;                       // support leg planted
      P.legB  = (0.60*wind - 2.20*sweep)*k; // windup -> full forward extension
      P.lean  = (0.14*wind + 0.38*sweep)*k; // torso leans back off the kick
      P.bob   = -0.05*sweep*k;
      P.armLp = -0.70*sweep*k;              // arms scissor for balance:
      P.armRp = 0.55*sweep*k;               // left forward, right back
      P.armOut = 0.16; P.elbow = 0.20;
      break;
    }

    case "stomp": {
      // The two-hit quick stomp finisher. `time` is seconds into it: the
      // stomping knee jerks up ahead of each scheduled impact (0.14s and
      // 0.34s, see FinisherKind::Stomp) and slams down ON it.
      const t = Math.max(time, 0);
      const pulse = (ti)=>
        Math.max(0, ss((t-(ti-0.13))/0.085) - ss((t-(ti-0.045))/0.045));
      const lift = Math.max(pulse(0.14), pulse(0.34));
      P.legA  = 0.10;                       // support leg planted
      P.legB  = -1.05*lift;                 // stomping knee hiked up forward
      P.lean  = -0.10*lift;                 // slight crouch over the body
      P.bob   = 0.05*lift - 0.02;
      P.armRaise = 0.35*lift;               // arms pump with each stomp
      P.armLp = 0.25; P.armRp = 0.25;
      break;
    }

    default: // "idle"-like neutral if unknown
      break;
  }
  return P;
}

/* compose local = translate * rot * scale, built parent-first (module helper) */
function part(parent, tx,ty,tz, rx,ry,rz, sx,sy,sz){
  let m = M4.translate(tx,ty,tz);
  if(rz) m = M4.mul(m, M4.rotZ(rz));
  if(ry) m = M4.mul(m, M4.rotY(ry));
  if(rx) m = M4.mul(m, M4.rotX(rx));
  const withScale = M4.mul(m, M4.scale(sx,sy,sz));
  return {node:parent?M4.mul(parent,m):m, draw:parent?M4.mul(parent,withScale):withScale};
}

/* ---------- SpritePipeline: the shared two-pass skeleton ----------
   Owns everything that is NOT scene-specific: the shader compile helpers, the
   fullscreen-quad buffer, the pass-1 scene target (an rt x rt RGBA texture +
   depth renderbuffer, alpha = part id) and the pass-2 "inked" post-process
   (edge-detect + posterize + pixelate) that resamples the scene into a target
   rect. RobotPipeline (below) and ShoggothPipeline (shoggoth-core.js) extend
   it with their own scene program + geometry and call:
     _beginScene()                      bind + clear the scene target, depth on
     _postPass(target, px, transparent) run pass 2 into `target` (see render())
   It does NOT own the context: it can live on a canvas of its own or inside a
   bigger renderer that draws many sprites per frame into its own framebuffers
   (the game's renderer.js).

   render() implementations leave these GL states behind, and restore nothing —
   the caller re-establishes whatever it needs afterwards:
     current program, ARRAY_BUFFER binding, the vertex-attrib arrays of the
     programs (enabled + pointed at their buffers), FRAMEBUFFER binding,
     viewport, clearColor, active texture unit (TEXTURE0) and the TEXTURE_2D
     bound on it, BLEND (disabled: pass 1 stores a part id in alpha, pass 2
     writes straight RGBA), DEPTH_TEST (disabled on exit; enabled during
     pass 1), CULL_FACE / SCISSOR_TEST (disabled). */
export class SpritePipeline {
  constructor(gl, rt, {edge=0.25} = {}){
    if(!gl) throw new Error("WebGL unavailable");
    this.gl = gl;
    this.edge = edge; // post-pass luma-gradient ink threshold

    this.postProg = this._program(postVS, postFS);
    this.pLoc = {
      aPos: gl.getAttribLocation(this.postProg,"aPos"),
      uTex: gl.getUniformLocation(this.postProg,"uTex"),
      uTexel: gl.getUniformLocation(this.postProg,"uTexel"),
      uPx: gl.getUniformLocation(this.postProg,"uPx"),
      uSize: gl.getUniformLocation(this.postProg,"uSize"),
      uTransparent: gl.getUniformLocation(this.postProg,"uTransparent"),
      uEdge: gl.getUniformLocation(this.postProg,"uEdge"),
      uAiInk: gl.getUniformLocation(this.postProg,"uAiInk"),
    };
    this.quadBuf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER,this.quadBuf);
    gl.bufferData(gl.ARRAY_BUFFER,new Float32Array([-1,-1, 1,-1, -1,1, -1,1, 1,-1, 1,1]),gl.STATIC_DRAW);

    this._buildTarget(rt);
  }

  _compile(type,src){
    const gl=this.gl;
    const s=gl.createShader(type);gl.shaderSource(s,src);gl.compileShader(s);
    if(!gl.getShaderParameter(s,gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s)+"\n"+src);
    return s;
  }
  _program(vs,fs){
    const gl=this.gl;
    const p=gl.createProgram();
    gl.attachShader(p,this._compile(gl.VERTEX_SHADER,vs));
    gl.attachShader(p,this._compile(gl.FRAGMENT_SHADER,fs));
    gl.linkProgram(p);
    if(!gl.getProgramParameter(p,gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(p));
    return p;
  }
  // Static geometry buffer helper (positions / normals).
  _staticBuffer(data){
    const gl=this.gl;
    const b=gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER,b); gl.bufferData(gl.ARRAY_BUFFER,data,gl.STATIC_DRAW);
    return b;
  }

  _buildTarget(RT){
    const gl=this.gl;
    this.RT = RT; // square pass-1 target
    this.rtTex = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, this.rtTex);
    gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA,RT,RT,0,gl.RGBA,gl.UNSIGNED_BYTE,null);
    gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_S,gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_T,gl.CLAMP_TO_EDGE);
    this.depthRB = gl.createRenderbuffer();
    gl.bindRenderbuffer(gl.RENDERBUFFER, this.depthRB);
    gl.renderbufferStorage(gl.RENDERBUFFER, gl.DEPTH_COMPONENT16, RT, RT);
    gl.bindRenderbuffer(gl.RENDERBUFFER, null);
    this.fbo = gl.createFramebuffer();
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.fbo);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, this.rtTex, 0);
    gl.framebufferRenderbuffer(gl.FRAMEBUFFER, gl.DEPTH_ATTACHMENT, gl.RENDERBUFFER, this.depthRB);
    if(gl.checkFramebufferStatus(gl.FRAMEBUFFER)!==gl.FRAMEBUFFER_COMPLETE) console.warn("FBO incomplete");
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
  }

  // Pass 1 prologue: the scene target is bound, cleared to (0,0,0,0) (alpha 0
  // = background id), depth test on, blending off. The subclass then binds its
  // scene program + geometry and draws.
  _beginScene(){
    const gl=this.gl;
    M4.reset();
    gl.disable(gl.BLEND);        // pass 1 alpha is a part id, pass 2 writes straight RGBA
    gl.disable(gl.CULL_FACE);
    gl.disable(gl.SCISSOR_TEST);
    // (unbind TEXTURE0 first: the previous pass 2 left our scene texture there,
    //  and it is this pass's color attachment — never sample-and-render at once)
    gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, null);
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.fbo);
    gl.viewport(0,0,this.RT,this.RT);
    gl.clearColor(0,0,0,0);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
    gl.enable(gl.DEPTH_TEST);
  }

  // Pass 2: post -> target rect (or the whole canvas).
  //   target (optional): {fbo, x, y, w, h} — the post pass is drawn into that
  //   viewport rect of `fbo` (null = the canvas). Every pixel of the rect is
  //   overwritten (background comes out as the dark floor color, or as
  //   (0,0,0,0) when transparent), so the caller never needs to clear it.
  //   Omitted -> the whole canvas (default framebuffer), cleared first.
  // `aiInk` (default 1) scales the part-id-boundary ink term — pass 0 for
  // tiny art where every texel borders a part and would come out black.
  _postPass(target, px, transparent, aiInk){
    const gl=this.gl;
    gl.disable(gl.DEPTH_TEST);
    if(target){
      gl.bindFramebuffer(gl.FRAMEBUFFER, target.fbo || null);
      gl.viewport(target.x|0, target.y|0, target.w|0, target.h|0);
    } else {
      gl.bindFramebuffer(gl.FRAMEBUFFER, null);
      gl.viewport(0,0,gl.drawingBufferWidth,gl.drawingBufferHeight);
      if(transparent){ gl.clearColor(0,0,0,0); } else { gl.clearColor(0.03,0.04,0.05,1); }
      gl.clear(gl.COLOR_BUFFER_BIT);
    }
    gl.useProgram(this.postProg);
    gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, this.rtTex);
    gl.uniform1i(this.pLoc.uTex,0);
    gl.uniform2f(this.pLoc.uTexel, 1/this.RT, 1/this.RT);
    gl.uniform2f(this.pLoc.uSize, this.RT, this.RT);
    gl.uniform1f(this.pLoc.uPx, Math.max(1, px || 5));
    gl.uniform1f(this.pLoc.uTransparent, transparent ? 1.0 : 0.0);
    gl.uniform1f(this.pLoc.uEdge, this.edge);
    gl.uniform1f(this.pLoc.uAiInk, aiInk === undefined ? 1.0 : aiInk);
    gl.bindBuffer(gl.ARRAY_BUFFER,this.quadBuf); gl.enableVertexAttribArray(this.pLoc.aPos); gl.vertexAttribPointer(this.pLoc.aPos,2,gl.FLOAT,false,0,0);
    gl.drawArrays(gl.TRIANGLES,0,6);
  }
}

/* ---------- the RobotPipeline: box-built robot on the shared skeleton ---------- */
class RobotPipeline extends SpritePipeline {
  constructor(gl, rt){
    super(gl, rt);
    this.sceneProg = this._program(sceneVS, sceneFS);
    this.sLoc = {
      aPos: gl.getAttribLocation(this.sceneProg,"aPos"),
      aNormal: gl.getAttribLocation(this.sceneProg,"aNormal"),
      aExtra: gl.getAttribLocation(this.sceneProg,"aExtra"),
      uVP: gl.getUniformLocation(this.sceneProg,"uVP"),
      uPart: gl.getUniformLocation(this.sceneProg,"uPart[0]"),
      uColors: gl.getUniformLocation(this.sceneProg,"uColors[0]"),
    };
    const mesh = buildMergedMesh();
    this.cubeVerts = mesh.vertCount;            // 36 verts per cube instance
    this.meshBuf = this._staticBuffer(mesh.data);
    this.heldRanges = mesh.held;                // weapon name -> {first, count}
    this.groundRanges = mesh.ground;            // weaponIdx  -> {first, count}
    this.palette = new Float32Array(PALETTE_PARTS * 16); // the mat4 uniform array
    // uColors: [body, accent, trim] filled per render; the fixed tail never changes
    this.colorTable = new Float32Array(24);
    this.colorTable.set(GUN_METAL, COL_METAL * 3);
    this.colorTable.set(GUN_DARK, COL_DARK * 3);
    this.colorTable.set(GROUND_ACCENT, COL_GROUND * 3);
    this.colorTable.set(GUN_LIGHT, COL_LIGHT * 3);
    this.colorTable.set(GUN_WOOD, COL_WOOD * 3);
  }

  // Bind the merged skinned mesh (interleaved pos3/nrm3/extra4, 40-byte stride).
  _bindMesh(){
    const gl=this.gl, sLoc=this.sLoc;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.meshBuf);
    gl.enableVertexAttribArray(sLoc.aPos);    gl.vertexAttribPointer(sLoc.aPos,   3,gl.FLOAT,false,40,0);
    gl.enableVertexAttribArray(sLoc.aNormal); gl.vertexAttribPointer(sLoc.aNormal,3,gl.FLOAT,false,40,12);
    gl.enableVertexAttribArray(sLoc.aExtra);  gl.vertexAttribPointer(sLoc.aExtra, 4,gl.FLOAT,false,40,24);
  }
  _unbindMesh(){
    // mirror the legacy exit state: aPos stays enabled for the post pass
    this.gl.disableVertexAttribArray(this.sLoc.aNormal);
    this.gl.disableVertexAttribArray(this.sLoc.aExtra);
  }
  // Upload the shared per-draw uniforms: view-projection, the color table and
  // the first `slots` pose matrices of the palette.
  _uploadUniforms(VP, slots){
    const gl=this.gl, sLoc=this.sLoc;
    gl.uniformMatrix4fv(sLoc.uVP, false, VP);
    gl.uniform3fv(sLoc.uColors, this.colorTable);
    gl.uniformMatrix4fv(sLoc.uPart, false, this.palette.subarray(0, slots*16));
  }

  // Fill the palette with this frame's pose matrices (the same matrices the
  // legacy per-part draws computed) and issue the 1-2 skinned draw calls.
  _renderRobot(VP, pal, plan, facingRad, weapon){
    const gl=this.gl;
    const P=this.palette, S=16;
    const recoil = plan.recoil || 0.0;
    const weaponParts = (weapon && weapon !== "fist") ? WEAPON_MODELS[weapon] : null;
    const holdingWeapon = !!(weaponParts && weaponParts.length);

    // root: face + backward-lean (flinch) + bob/recoil offsets
    let root = M4.mul(M4.translate(0, plan.bob, plan.zback), M4.rotY(facingRad));
    if(plan.lean) root = M4.mul(root, M4.rotX(plan.lean));

    // torso / head / visor strip / hips -> palette slots 0-3
    P.set(part(root, 0,1.15,0, 0,0,0, 0.9,1.0,0.55).draw, 0*S);
    P.set(part(root, 0,1.95,0.02, 0,0,0, 0.62,0.55,0.55).draw, 1*S);
    P.set(part(root, 0,1.98,0.28, 0,0,0, 0.5,0.16,0.08).draw, 2*S);
    P.set(part(root, 0,0.72,0, 0,0,0, 0.8,0.3,0.5).draw, 3*S);
    if(plan.headless){
      // decapitated (the "downed_headless" pose): collapse the head + visor
      // cubes to zero — degenerate triangles rasterize nothing, so the one
      // merged draw below stays a single call.
      P.fill(0, 1*S, 3*S);
    }

    // legs (pivot at hip, swing around X so they step fwd/back along Z)
    function leg(sideX, ph, slot){
      const hipPivot = M4.mul(root, M4.translate(sideX,0.6,0));
      const swung = M4.mul(hipPivot, M4.rotX(ph));
      const thigh = M4.mul(swung, M4.mul(M4.translate(0,-0.28,0), M4.scale(0.3,0.62,0.32)));
      P.set(thigh, slot*S);
      const knee = M4.mul(swung, M4.translate(0,-0.6,0));
      const shinRot = M4.mul(knee, M4.rotX(Math.max(0,-ph)*0.6));
      const shin = M4.mul(shinRot, M4.mul(M4.translate(0,-0.28,0), M4.scale(0.26,0.6,0.28)));
      P.set(shin, (slot+1)*S);
      const foot = M4.mul(shinRot, M4.mul(M4.translate(0,-0.6,0.06), M4.scale(0.32,0.22,0.5)));
      P.set(foot, (slot+2)*S);
    }
    leg(-0.32, plan.legA, 4);
    leg( 0.32, plan.legB, 7);

    // arms (pivot at shoulder); the gun-hand grows the held weapon (slots 15+)
    // or the bare-hand barrel (slot 14)
    let barrelDrawn = false;
    function arm(sideX, ph, forward, gunHand, slot){
      const shoulder = M4.mul(root, M4.translate(sideX,1.5,0));
      let rot;
      if(forward){
        rot = M4.mul(shoulder, M4.rotX(-1.35 + recoil));
      } else {
        rot = M4.mul(shoulder, M4.rotX(ph - plan.armRaise));
        if(plan.armOut){
          // relaxed hang: splay the whole arm slightly outward from the hips
          rot = M4.mul(rot, M4.rotZ(sideX > 0 ? plan.armOut : -plan.armOut));
        }
      }
      const upper = M4.mul(rot, M4.mul(M4.translate(0,-0.26,0), M4.scale(0.24,0.55,0.26)));
      P.set(upper, slot*S);
      let elbow = M4.mul(rot, M4.translate(0,-0.52,0));
      if(!forward && plan.elbow){
        // relaxed hang: a soft natural bend at the elbow
        elbow = M4.mul(elbow, M4.rotX(plan.elbow));
      }
      const fore = M4.mul(elbow, M4.mul(M4.translate(0,-0.24,0), M4.scale(0.2,0.5,0.22)));
      P.set(fore, (slot+1)*S);
      if(gunHand && holdingWeapon){
        // a held weapon replaces the bare-hand barrel; anchored at the grip
        const anchor = M4.mul(elbow, M4.translate(0, -0.42, 0.0));
        for(let j=0;j<weaponParts.length;j++){
          const b = weaponParts[j];
          P.set(M4.mul(anchor, M4.mul(M4.translate(b.t[0],b.t[1],b.t[2]),
                                      M4.scale(b.s[0],b.s[1],b.s[2]))), (WEAPON_SLOT0+j)*S);
        }
      } else if(forward){
        const barrel = M4.mul(elbow, M4.mul(M4.translate(0,-0.5,0.0), M4.scale(0.14,0.5,0.14)));
        P.set(barrel, 14*S);
        barrelDrawn = true;
      }
    }
    // the right arm is the gun-hand: force it forward whenever a weapon is held,
    // so the weapon sticks out in front (like the shoot pose) from every pose.
    arm(-0.62, plan.armLp, false, false, 10);
    arm( 0.62, plan.armRp, plan.shoot || holdingWeapon, true, 12);

    this.colorTable.set(pal.body, COL_BODY*3);
    this.colorTable.set(pal.accent, COL_ACCENT*3);
    this.colorTable.set(pal.trim, COL_TRIM*3);
    const slots = holdingWeapon ? WEAPON_SLOT0 + weaponParts.length
                                : (barrelDrawn ? 15 : 14);
    this._uploadUniforms(VP, slots);
    // body (+ the barrel, which sits right after it in the buffer) ...
    gl.drawArrays(gl.TRIANGLES, 0, (barrelDrawn ? BODY_CUBES+1 : BODY_CUBES) * this.cubeVerts);
    // ... then the held weapon: everything after the forearms, like before
    if(holdingWeapon){
      const r = this.heldRanges[weapon];
      gl.drawArrays(gl.TRIANGLES, r.first, r.count);
    }
  }

  /* render one robot.
     opts: {pose, color|pal, px, time, facingDeg, weapon, transparent,
            orbit:{yaw,pitch,halfV,center}, halfV}
       weapon: one of WEAPONS ("fist" | "pistol" | "machinegun" | "shotgun")
       time:   continuous seconds — every value renders a distinct frame
     target (optional): {fbo, x, y, w, h} — see SpritePipeline._postPass. */
  render(opts, target){
    const gl=this.gl;
    const pose = (opts.pose || "idle");
    const pal  = opts.pal || PALETTES[(opts.color||"coral")] || PALETTES.coral;
    const time = opts.time || 0;
    const weapon = (opts.weapon in WEAPON_MODELS) ? opts.weapon : "fist";
    const facingRad = (opts.facingDeg || 0) * Math.PI/180;

    // pass 1: scene -> FBO
    this._beginScene();
    const VP = opts.orbit
      ? orbitVP(opts.orbit.yaw||0, opts.orbit.pitch||0, opts.orbit.halfV, opts.orbit.center)
      : topDownVP(opts.halfV);
    // Unarmed robots stand / walk at ease rather than in the combat rig.
    const plan = posePlan(pose, time, weapon === "fist");
    gl.useProgram(this.sceneProg);
    this._bindMesh();
    this._renderRobot(VP, pal, plan, facingRad, weapon);
    this._unbindMesh();

    // pass 2: post -> target rect (or the whole canvas)
    this._postPass(target, opts.px, !!opts.transparent);
  }

  /* render one WEAPON lying flat on the ground (a pickup, or a thrown weapon
     spinning across the floor), seen by the same true top-down camera as the
     robots so it composes into the world at matching perspective. The model
     is laid on its side (thickness axis up -> the recognizable side profile
     faces the camera) and rotated by `angle` around the vertical axis.
     opts: {weaponIdx (0 bar / 1 pistol / 2 machinegun / 3 shotgun),
            angle (radians, screen convention: 0 = muzzle toward +x,
            positive = clockwise on screen), px, transparent, halfV}
     target: see SpritePipeline._postPass. */
  renderGun(opts, target){
    const gl=this.gl;
    const idx = GROUND_WEAPON_MODELS[opts.weaponIdx|0] ? (opts.weaponIdx|0) : 0;
    const parts = GROUND_WEAPON_MODELS[idx];

    this._beginScene();
    const VP = topDownVP(opts.halfV || 0.72);
    gl.useProgram(this.sceneProg);
    this._bindMesh();

    // Ground frame: lift to the camera's focus height, spin around vertical
    // (negated: world +z is screen +y/down, so -yaw = clockwise on screen),
    // lay the model on its side (gun local X/thickness -> world up, the
    // barrel's local -Y -> world +x) and centre it on its midpoint.
    // The ground vertices carry ONE shared part id (0.9): at the tiny
    // ground-art resolution every texel neighbours a part boundary, so
    // per-part ids would ink the whole weapon black. The silhouette pass
    // still outlines it. Colors: gunmetal + the fixed GROUND_ACCENT.
    let base = M4.mul(M4.translate(0,0.9,0), M4.rotY(-(opts.angle||0)));
    base = M4.mul(base, M4.mul(M4.rotZ(Math.PI/2), M4.translate(0, GROUND_WEAPON_CENTER[idx]||0, 0)));
    const P=this.palette;
    for(let j=0;j<parts.length;j++){
      const b = parts[j];
      P.set(M4.mul(base, M4.mul(M4.translate(b.t[0],b.t[1],b.t[2]),
                                M4.scale(b.s[0],b.s[1],b.s[2]))), j*16);
    }
    this._uploadUniforms(VP, parts.length);
    const r = this.groundRanges[idx];
    gl.drawArrays(gl.TRIANGLES, r.first, r.count);
    this._unbindMesh();

    // No interior linework at all (the art is a handful of texels): raise the
    // luma-ink threshold out of reach and disable the id-boundary ink; only
    // the outer silhouette ring survives, which keeps the weapon readable on
    // any floor without eating its fill.
    const prevEdge = this.edge;
    this.edge = 9.0;
    this._postPass(target, opts.px, !!opts.transparent, 0.0);
    this.edge = prevEdge;
  }

  /* render one DETACHED HEAD lying on the ground (a KICK finisher trophy):
     the head + visor cubes only, seen by the same true top-down camera as
     the robots, laid FACE-UP so the camera sees the visor — a decapitated
     head staring at the ceiling. Baked once per colour at angle 0 (the
     caller's cache) and spun as a 2D quad, exactly like the ground guns:
     under the straight-down ortho the two are equivalent.
     opts: {color|pal, px, transparent, halfV}
     target: see SpritePipeline._postPass. */
  renderHead(opts, target){
    const gl=this.gl;
    const pal = opts.pal || PALETTES[(opts.color||"coral")] || PALETTES.coral;

    this._beginScene();
    const VP = topDownVP(opts.halfV || 0.55);
    gl.useProgram(this.sceneProg);
    this._bindMesh();

    // Ground frame: lift the head centre to the camera's focus height and
    // pitch the face (local +Z, the visor side) up toward world +Y — tipped
    // ~0.4 rad short of straight up, so the camera sees the visor band AND
    // a sliver of the head's crown (the tilt is what makes it read as a
    // decapitated head staring at the ceiling, not a plain box). Cube
    // offsets/scales are the body rig's own head (slot 1) and visor strip
    // (slot 2) values, re-anchored at the head; the visor is nudged a bit
    // proud of the face so it always wins the depth test in the bake.
    const base = M4.mul(M4.translate(0,0.9,0), M4.rotX(-Math.PI/2 + 0.4));
    const P=this.palette;
    P.set(M4.mul(base, M4.scale(0.62,0.55,0.55)), 1*16);
    P.set(M4.mul(base, M4.mul(M4.translate(0,0.03,0.29), M4.scale(0.5,0.16,0.12))), 2*16);
    this.colorTable.set(pal.body, COL_BODY*3);
    this.colorTable.set(pal.accent, COL_ACCENT*3);
    this.colorTable.set(pal.trim, COL_TRIM*3);
    this._uploadUniforms(VP, 3);
    // the head + visor cubes sit contiguously after the torso in the buffer
    gl.drawArrays(gl.TRIANGLES, this.cubeVerts, this.cubeVerts*2);
    this._unbindMesh();

    // Tiny art: silhouette-only ink, like the ground guns.
    const prevEdge = this.edge;
    this.edge = 9.0;
    this._postPass(target, opts.px, !!opts.transparent, 0.0);
    this.edge = prevEdge;
  }
}

/* createRobotPipeline(gl, {rt}) — the pipeline on a caller-owned context.
   rt: pass-1 scene resolution in px (square); the post pass resamples it into
   whatever target rect render() is given, so rt is the detail budget, not the
   output size. */
export function createRobotPipeline(gl, {rt=128} = {}){ return new RobotPipeline(gl, rt); }

/* ---------- CanvasRenderer: a pipeline bound to one canvas of its own ----------
   makePipeline(gl, rt) builds the pipeline (square target at canvas res). */
export class CanvasRenderer {
  constructor(canvas, makePipeline){
    this.canvas = canvas;
    const gl = canvas.getContext("webgl", {antialias:false, preserveDrawingBuffer:true});
    if(!gl) throw new Error("WebGL unavailable");
    this.gl = gl;
    this.pipeline = makePipeline(gl, canvas.width);
  }
  /* render one frame to this canvas — see the pipeline's render for opts. */
  render(opts){ this.pipeline.render(opts, null); }
}
const makeRobotPipeline = (gl, rt) => new RobotPipeline(gl, rt);
export function createRenderer(canvas){ return new CanvasRenderer(canvas, makeRobotPipeline); }

/* ---------- makeBaker: "one baked frame as a standalone canvas" factory ----------
   Returns bake(opts, size) -> HTMLCanvasElement: renders one frame through a
   shared internal canvas renderer (so repeated calls do not leak GL contexts)
   and copies it into a fresh canvas the caller owns. */
export function makeBaker(makePipeline){
  let renderer = null, rSize = 0;
  return function bake(opts, size){
    if(!renderer || rSize !== size){
      const c = document.createElement("canvas");
      c.width = c.height = size;
      renderer = new CanvasRenderer(c, makePipeline);
      rSize = size;
    }
    renderer.render(opts);
    const out = document.createElement("canvas");
    out.width = out.height = size;
    out.getContext("2d").drawImage(renderer.canvas, 0, 0);
    return out;
  };
}

/* ---------- bakeSprite: one baked robot frame as a standalone canvas ----------
   Renders ONE baked, top-down, inked/pixelated sprite frame. (The game itself
   does not bake: it runs createRobotPipeline() live inside its own context.) */
const _bakeRobot = makeBaker(makeRobotPipeline);
export function bakeSprite({pose="idle", color="coral", facingDeg=0, px=5, time=0, size=384, weapon="fist", transparent=false} = {}){
  return _bakeRobot({pose, color, px, time, facingDeg, weapon, transparent}, size); // top-down (no orbit)
}
