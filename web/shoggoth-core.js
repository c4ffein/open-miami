"use strict";
/* =========================================================================
   OPEN MIAMI - the FLOOR 13½ boss: the SHOGGOTH, 3D -> stylized 2D.
   Vanilla WebGL 1, no libraries. Built on robot-core.js's SpritePipeline
   (same pass-1 target, same "inked" edge/posterize/pixelate post pass, same
   pooled mat4 helpers) — only the scene is its own: shaded SPHERES.

   THERE IS NO ANIMATION HERE. What the boss looks like at time t — the
   writhing mass, the lashing tentacles, the yellow mask cracking and being
   swallowed, even the mask's "look up" beats — is computed by the ENGINE
   (src/render/shoggoth.rs `boss_spheres`) as a list of spheres, 20 floats
   each. The game ships it as a run of SPHERE ops closed by SHOGGOTH; the
   tool pages ask the wasm (tools/engine-pose.js `bossSpheres`). This file
   owns the CAMERA, the SHADING and the two draw paths.

   Exports:
     SPHERE_FLOATS                     - 20: floats per sphere (= instVS)
     SPHERE_TESS / DEFAULT_TESS        - sphere tessellation presets
                                         ("high" 16x20, "low" 8x10; the game
                                         default is LOW)
     createShoggothPipeline(gl, {rt, tess}) - the pipeline on an EXISTING GL
         context:
         .render(opts, target) draws one shoggoth into a caller-provided
         framebuffer rect {fbo,x,y,w,h} (or the whole canvas when omitted),
         transparent background when opts.transparent. This is what the game
         renderer runs live, every frame, inside its own GL context.
           opts: {
             spheres: REQUIRED — { data: Float32Array, n, maskAt }: `n`
                      spheres of SPHERE_FLOATS floats in `data` (the model's
                      rows 0..2, rgb + part id, accent rgb + emission);
                      spheres [0, maskAt) draw with the depth test ON, the
                      rest — the mask assembly — with it OFF, in order
             px:      pixelation block size (post pass), default 5
             tess:    "low" | "high" — switch the sphere preset live
                      (rebuilds the shared buffers only when it changes)
             transparent, orbit:{yaw,pitch,halfV}, halfV — as robot-core
           }
         .instanced = false forces the per-sphere REFERENCE path
     createShoggothRenderer(canvas)    - a CanvasRenderer of the pipeline
     bakeShoggoth({...opts, size})     - one baked top-down frame -> canvas
   ========================================================================= */

import { M4, SpritePipeline, CanvasRenderer, makeBaker, orbitVP } from "./robot-core.js";

/* ---------- scene shader: lit surfaces + per-part id in alpha (for edges),
   plus an emissive term so the yellow mask and eyes glow flat through the
   posterize. Same lighting model as robot-core's scene shader. ---------- */
const sceneVS = `
attribute vec3 aPos;
attribute vec3 aNormal;
uniform mat4 uMVP;
uniform mat3 uNormalMat;
varying vec3 vN;
void main(){
  vec4 p = uMVP * vec4(aPos,1.0);
  gl_Position = p;
  vN = normalize(uNormalMat * aNormal);
}
`;
const sceneFS = `
precision mediump float;
varying vec3 vN;
uniform vec3 uColor;
uniform vec3 uAccent;
uniform float uId;
uniform float uEmis;   // 0 = fully lit, 1 = flat glow color
void main(){
  vec3 L = normalize(vec3(0.35, 0.9, 0.45));
  float ndl = max(dot(normalize(vN), L), 0.0);
  float amb = 0.35;
  float shade = amb + ndl*0.75;
  vec3 base = mix(uColor, uAccent, clamp(vN.y*0.5+0.2,0.0,1.0)*0.5);
  vec3 col = base * shade;
  col = mix(col, uColor, uEmis);
  gl_FragColor = vec4(col, uId);
}
`;

/* ---------- unit sphere geometry (positions + normals) ---------- */
/* THE INSTANCED PATH (what the game runs). The boss is nothing but spheres —
   placed by the engine (src/render/shoggoth.rs) — so a
   sphere is 20 floats of per-INSTANCE data (the model's three rows + two
   colour vec4s) and the whole boss is TWO instanced draws instead of one draw
   + six uniform uploads per sphere (a few dozen of them): the body with the
   depth test on, then the mask assembly with it off, in submission order —
   exactly what the per-sphere path does. Same fragment shader, same maths:
     - gl_Position = uVP * (model * pos): the reference multiplies VP * model in
       JS doubles and uploads the product; here the GPU does it in float32, so
       the two agree to a few edge texels (tests/e2e/render/shoggoth-parity.js);
     - the "normal matrix" is the model's upper 3x3 AS IS (M4.normalFromModel —
       not an inverse-transpose; the look was tuned with it), so it is derived
       from the same three rows.
   7 vertex attributes in all (WebGL 1 guarantees 8). Needs
   ANGLE_instanced_arrays; without it the per-sphere REFERENCE path serves, and
   `pipe.instanced = false` forces it (the parity page's A/B switch). */
export const SPHERE_FLOATS = 20;
const instVS = `
attribute vec3 aPos;
attribute vec3 aNormal;
attribute vec4 aM0;     // model row 0
attribute vec4 aM1;     // model row 1
attribute vec4 aM2;     // model row 2
attribute vec4 aColId;  // rgb colour, a = part id
attribute vec4 aAccEm;  // rgb accent, a = emission
uniform mat4 uVP;
varying vec3 vN;
varying vec4 vColId;
varying vec4 vAccEm;
void main(){
  vec4 p = vec4(aPos, 1.0);
  vec4 w = vec4(dot(aM0, p), dot(aM1, p), dot(aM2, p), 1.0);
  gl_Position = uVP * w;
  vN = normalize(vec3(dot(aM0.xyz, aNormal), dot(aM1.xyz, aNormal), dot(aM2.xyz, aNormal)));
  vColId = aColId;
  vAccEm = aAccEm;
}
`;
const instFS = `
precision mediump float;
varying vec3 vN;
varying vec4 vColId;
varying vec4 vAccEm;
void main(){
  vec3 uColor = vColId.rgb;
  vec3 uAccent = vAccEm.rgb;
  float uEmis = vAccEm.a;
  vec3 L = normalize(vec3(0.35, 0.9, 0.45));
  float ndl = max(dot(normalize(vN), L), 0.0);
  float amb = 0.35;
  float shade = amb + ndl*0.75;
  vec3 base = mix(uColor, uAccent, clamp(vN.y*0.5+0.2,0.0,1.0)*0.5);
  vec3 col = base * shade;
  col = mix(col, uColor, uEmis);
  gl_FragColor = vec4(col, vColId.a);
}
`;

function makeSphere(stacks, slices){
  const p=[], n=[];
  function vert(i,j){
    const v = i/stacks, u = j/slices;
    const phi = v*Math.PI;
    const th  = u*2*Math.PI;
    return [Math.sin(phi)*Math.cos(th), Math.cos(phi), Math.sin(phi)*Math.sin(th)];
  }
  for(let i=0;i<stacks;i++){
    for(let j=0;j<slices;j++){
      const a=vert(i,j), b=vert(i,j+1), c=vert(i+1,j+1), d=vert(i+1,j);
      for(const v of [a,b,c, a,c,d]){ p.push(v[0],v[1],v[2]); n.push(v[0],v[1],v[2]); }
    }
  }
  return {pos:new Float32Array(p), nrm:new Float32Array(n), count:p.length/3};
}

/* Sphere tessellation presets. The boss is ~130 sphere instances per frame,
   all inked + pixelated into a small tile, so LOW (8x10) is visually
   identical to HIGH (the legacy 16x20) at game size for ~1/4 the vertices.
   The GAME default is LOW; the inspector has a TESS toggle to compare
   (render opts.tess or createShoggothPipeline's {tess}). */
export const SPHERE_TESS = { high:[16,20], low:[8,10] };
export const DEFAULT_TESS = "low";

/* ---------- camera: slightly-tilted top-down (the boss reads best with a hint
   of the mask's face). The half-extent frames the whole raw form: the mass
   (radius ~2.5 with its lobes) plus most of the tentacles' reach, so the
   in-game tile does not cut them off square. ---------- */
const CAM_HALF_V = 3.8;
const CAM_CENTER = [0,0.4,0];
function bossVP(halfV){
  halfV = halfV || CAM_HALF_V;
  const proj = M4.ortho(-halfV,halfV,-halfV,halfV,0.1,60);
  const eye=[0, 12, 6.1], up=[0,0,-1];
  return M4.mul(proj, M4.lookAt(eye,CAM_CENTER,up));
}

/* =========================================================================
   The pipeline
   ========================================================================= */
class ShoggothPipeline extends SpritePipeline {
  constructor(gl, rt, tess){
    super(gl, rt, {edge:0.30});
    this.sceneProg = this._program(sceneVS, sceneFS);
    this.sLoc = {
      aPos: gl.getAttribLocation(this.sceneProg,"aPos"),
      aNormal: gl.getAttribLocation(this.sceneProg,"aNormal"),
      uMVP: gl.getUniformLocation(this.sceneProg,"uMVP"),
      uNormalMat: gl.getUniformLocation(this.sceneProg,"uNormalMat"),
      uColor: gl.getUniformLocation(this.sceneProg,"uColor"),
      uAccent: gl.getUniformLocation(this.sceneProg,"uAccent"),
      uId: gl.getUniformLocation(this.sceneProg,"uId"),
      uEmis: gl.getUniformLocation(this.sceneProg,"uEmis"),
    };
    this.tess = null;
    this.setTess(tess || DEFAULT_TESS);
    this.VP = null;

    // the instanced path (see instVS): on by default when the extension is there
    this.instExt = gl.getExtension("ANGLE_instanced_arrays");
    this.instanced = !!this.instExt;
    if(this.instExt){
      this.instProg = this._program(instVS, instFS);
      this.iLoc = {
        aPos: gl.getAttribLocation(this.instProg,"aPos"),
        aNormal: gl.getAttribLocation(this.instProg,"aNormal"),
        inst: ["aM0","aM1","aM2","aColId","aAccEm"].map(n => gl.getAttribLocation(this.instProg, n)),
        uVP: gl.getUniformLocation(this.instProg,"uVP"),
      };
      this.instBuf = gl.createBuffer();
    }
    // reference-path scratch (no per-sphere allocation)
    this._model = new Float32Array(16); this._model[15] = 1;
    this._col = new Float32Array(3); this._acc = new Float32Array(3);
  }

  /* switch the sphere tessellation preset ("low" | "high"); rebuilds the
     shared unit-sphere buffers, no-op when already on that preset. */
  setTess(name){
    if(!SPHERE_TESS[name]) name = DEFAULT_TESS;
    if(name === this.tess) return;
    const gl=this.gl;
    if(this.posBuf) gl.deleteBuffer(this.posBuf);
    if(this.nrmBuf) gl.deleteBuffer(this.nrmBuf);
    this.tess = name;
    const [stacks, slices] = SPHERE_TESS[name];
    this.sphere = makeSphere(stacks, slices);
    this.posBuf = this._staticBuffer(this.sphere.pos);
    this.nrmBuf = this._staticBuffer(this.sphere.nrm);
  }

  /* THE INSTANCED PATH: the engine's sphere list IS the instance buffer —
     uploaded as is, then the body (depth test ON) and the mask assembly (OFF,
     submission order). Leaves every instanced attribute disabled with its
     divisor back at 0: a divisor is per attribute INDEX, global to the
     context — leaking one would poison whichever program the renderer binds
     next. */
  _drawInstanced(sp){
    const gl=this.gl, ext=this.instExt, L=this.iLoc, n=sp.n;
    const split = Math.min(Math.max(sp.maskAt|0, 0), n);
    if(n <= 0) return;
    const stride = SPHERE_FLOATS * 4;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.instBuf);
    gl.bufferData(gl.ARRAY_BUFFER, sp.data.subarray(0, n * SPHERE_FLOATS), gl.DYNAMIC_DRAW);
    const range = (first, count) => {
      if(count <= 0) return;
      for(let k=0;k<5;k++){
        gl.enableVertexAttribArray(L.inst[k]);
        gl.vertexAttribPointer(L.inst[k], 4, gl.FLOAT, false, stride, first*stride + k*16);
        ext.vertexAttribDivisorANGLE(L.inst[k], 1);
      }
      ext.drawArraysInstancedANGLE(gl.TRIANGLES, 0, this.sphere.count, count);
    };
    range(0, split);
    if(n > split){
      gl.disable(gl.DEPTH_TEST);
      range(split, n - split);
      gl.enable(gl.DEPTH_TEST);
    }
    for(let k=0;k<5;k++){
      ext.vertexAttribDivisorANGLE(L.inst[k], 0);
      gl.disableVertexAttribArray(L.inst[k]);
    }
  }

  /* THE REFERENCE PATH: one draw + six uniform uploads per sphere, from the
     same list (row r of the model is data[o+4r .. o+4r+3]; M4 is column-major).
     uMVP = VP * model in JS, the normal matrix = the model's upper 3x3 as is. */
  _drawReference(sp){
    const gl=this.gl, sLoc=this.sLoc, D=sp.data, m=this._model, n=sp.n;
    const split = Math.min(Math.max(sp.maskAt|0, 0), n);
    for(let i=0;i<n;i++){
      if(i === split) gl.disable(gl.DEPTH_TEST);
      const o = i * SPHERE_FLOATS;
      m[0]=D[o];   m[4]=D[o+1]; m[8]=D[o+2];   m[12]=D[o+3];
      m[1]=D[o+4]; m[5]=D[o+5]; m[9]=D[o+6];   m[13]=D[o+7];
      m[2]=D[o+8]; m[6]=D[o+9]; m[10]=D[o+10]; m[14]=D[o+11];
      this._col[0]=D[o+12]; this._col[1]=D[o+13]; this._col[2]=D[o+14];
      this._acc[0]=D[o+16]; this._acc[1]=D[o+17]; this._acc[2]=D[o+18];
      gl.uniformMatrix4fv(sLoc.uMVP, false, M4.mul(this.VP, m));
      gl.uniformMatrix3fv(sLoc.uNormalMat, false, M4.normalFromModel(m));
      gl.uniform3fv(sLoc.uColor, this._col);
      gl.uniform3fv(sLoc.uAccent, this._acc);
      gl.uniform1f(sLoc.uId, D[o+15]);
      gl.uniform1f(sLoc.uEmis, D[o+19]);
      gl.drawArrays(gl.TRIANGLES, 0, this.sphere.count);
    }
    if(n > split) gl.enable(gl.DEPTH_TEST);
  }

  /* render one shoggoth — opts / target: see the module header. */
  render(opts, target){
    const gl=this.gl;
    if(opts.tess) this.setTess(opts.tess);
    const sp = opts.spheres;
    if(!sp || !sp.data) throw new Error("shoggoth-core: opts.spheres is required — the boss is placed by the engine (src/render/shoggoth.rs); tools get it from tools/engine-pose.js");

    // pass 1: scene -> FBO
    this._beginScene();
    this.VP = opts.orbit
      ? orbitVP(opts.orbit.yaw||0, opts.orbit.pitch||0, opts.orbit.halfV||CAM_HALF_V, CAM_CENTER)
      : bossVP(opts.halfV);
    const inst = this.instanced && !!this.instExt;
    const loc = inst ? this.iLoc : this.sLoc;
    gl.useProgram(inst ? this.instProg : this.sceneProg);
    gl.bindBuffer(gl.ARRAY_BUFFER,this.posBuf); gl.enableVertexAttribArray(loc.aPos); gl.vertexAttribPointer(loc.aPos,3,gl.FLOAT,false,0,0);
    gl.bindBuffer(gl.ARRAY_BUFFER,this.nrmBuf); gl.enableVertexAttribArray(loc.aNormal); gl.vertexAttribPointer(loc.aNormal,3,gl.FLOAT,false,0,0);
    if(inst){
      gl.uniformMatrix4fv(this.iLoc.uVP, false, this.VP);
      this._drawInstanced(sp);
    } else {
      this._drawReference(sp);
    }
    gl.disableVertexAttribArray(loc.aNormal);

    // pass 2: post -> target rect (or the whole canvas)
    this._postPass(target, opts.px, !!opts.transparent);
  }
}

/* createShoggothPipeline(gl, {rt}) — the pipeline on a caller-owned context.
   rt: pass-1 scene resolution in px (square); the post pass resamples it into
   whatever target rect render() is given, so rt is the detail budget, not the
   output size. The boss is big: 256 is a good default. */
export function createShoggothPipeline(gl, {rt=256, tess=DEFAULT_TESS} = {}){ return new ShoggothPipeline(gl, rt, tess); }

const makeShoggothPipeline = (gl, rt) => new ShoggothPipeline(gl, rt);
/* a CanvasRenderer bound to one canvas (owns a context + a pipeline) */
export function createShoggothRenderer(canvas){ return new CanvasRenderer(canvas, makeShoggothPipeline); }

/* bakeShoggoth({spheres, px, tess, size, transparent})
   -> HTMLCanvasElement: ONE baked top-down frame (the inspector's 2D view). */
const _bakeShoggoth = makeBaker(makeShoggothPipeline);
export function bakeShoggoth({size=384, ...opts} = {}){
  const o = Object.assign({}, opts);
  delete o.orbit; // top-down only
  return _bakeShoggoth(o, size);
}
