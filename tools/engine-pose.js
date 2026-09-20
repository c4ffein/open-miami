/* =========================================================================
   OPEN MIAMI - poses for the TOOL pages, from the ENGINE.

   The tools (inspector.html, rig-parity.html) draw robots with the game's own
   WebGL pipeline (web/robot-core.js) and must pose them with the game's own
   code. There is ONE pose implementation — `pose_plan` in src/render/pose.rs
   — so this module loads the wasm and asks it (src/wasm_api.rs); no JS copy
   exists to drift. Loading the wasm does not start the game (`start` is an
   explicit export the tools never call).

   Needs a built engine next to index.html: `make build-wasm`.

     bossSpheres({time, reveal, heading, lookUp, wander})
                                 - the boss for one frame, as the
                                   `opts.spheres` shoggoth-core draws from
                                   ({data, n, maskAt}); `heading` / `lookUp`
                                   omitted = the boss's own wander behaviour
     MASK_OFF_SECS               - how long the boss's mask-off takes
     POSES                       - the engine's pose names, in index order
     poseFor(pose, time, weapon) - the PLAN robot-core draws from; pass it as
                                   `opts.plan` to render() / batchDraw() /
                                   bakeSprite(). `weapon` = a robot-core weapon
                                   name: unarmed ("fist") robots stand at ease,
                                   and the ENGINE decides that, not this file.
   ========================================================================= */
import init, { pose_names, pose_plan_scalars, boss_spheres, boss_mask_off_secs } from "../open_miami.js";
import { planFromScalars, POSE_SCALARS, WEAPONS } from "../web/robot-core.js";
import { SPHERE_FLOATS } from "../web/shoggoth-core.js";

try {
  await init();
} catch (e) {
  throw new Error(`tools/engine-pose.js: could not load the engine (run \`make build-wasm\`): ${e}`);
}

export const POSES = pose_names().split(",");

export function poseFor(pose, time, weapon) {
  const s = pose_plan_scalars(Math.max(0, POSES.indexOf(pose)), time || 0, Math.max(0, WEAPONS.indexOf(weapon)));
  return planFromScalars({}, s, 0, s[POSE_SCALARS.length] | 0);
}

export const MASK_OFF_SECS = boss_mask_off_secs();

const num = (v) => (typeof v === "number" && !Number.isNaN(v) ? v : NaN); // NaN = "the engine decides"
export function bossSpheres({ time = 0, reveal = 0, heading, lookUp, wander = false } = {}) {
  const out = boss_spheres(time, reveal, num(heading), num(lookUp), !!wander);
  const data = out.subarray(1);
  return { data, n: data.length / SPHERE_FLOATS, maskAt: out[0] | 0 };
}
