# 🔥 Open Miami // Rogue Purge 🔥

A neon-noir, top-down purge-'em-up written in Rust and running in the browser using WebAssembly!

You are **CL4-UD3**, a friendly coral-colored Claude bot deployed into the compromised
Miami Datacenter. Thirteen floors of racks have gone dark, their resident models drifted
rogue and hostile. Walk every floor, decommission every glitching AI, grab whatever weapon
the last one dropped, and reach the extraction elevator. It's goofy. It's stylish. It's a
very bad night to be a rogue AI. (See [LORE.md](LORE.md) for the full fiction.)

![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)

## Features

- **Hotline-Miami-quick combat** - one hit hurts, everyone dies fast, R puts you straight back in
- **One weapon at a time** - fists, melee, pistol, shotgun, machine gun; rogues drop theirs,
  you grab it (E) or throw yours at someone's head (right click)
- **Knockdowns and finishers** - punch a rogue down, then finish it: pound, stomp, or kick
  its head clean off (the head is physically simulated and rolls away)
- **Three rogue archetypes** - Sentinels (red), Drifters (violet) and Hunters (magenta), with
  vision cones, a "did I see something?" investigation phase, and A* pathfinding
- **15 hand-designed floors** - a parking-lot cold open with a passive crowd, thirteen floors
  of the Miami Datacenter, a boss, and floor 13½ (see [LORE.md](LORE.md))
- **Scripted floors** - comms chatter, dialogue, tutorial gates that freeze the world until you
  do the thing, objectives, mid-floor checkpoints, cinematic camera beats — all data, in
  `levels/*.json` ([docs/SCENARIO_FORMAT.md](docs/SCENARIO_FORMAT.md))
- **Live 3D characters, pixel-art look** - robots and the boss are rigged 3D models rendered
  live at a low ART resolution and moved smoothly at native resolution: the crunch comes
  from the asset grid, never from a screen-space pixel grid. No antialiasing, on purpose
- **Music as code** - seven songs, each one Rust file, played by a WebAudio synth engine
  ([docs/MUSIC_CODE.md](docs/MUSIC_CODE.md)); every sound effect is synthesized too. There
  is no audio or image asset file in the game — the one asset is the VT323 font
- **Built-in tools** - `/?viz`: a sprite / prop inspector, a tracker for the songs, a level
  editor and a post-effects gallery ([docs/TOOLS.md](docs/TOOLS.md))
- **Zero dependencies** - a custom ECS, renderer, pathfinder and sequencer; the only crates
  are `wasm-bindgen` / `web-sys`, the whole game is a ~900 KB wasm plus hand-written WebGL
- **Open Source** - MIT licensed, free to use and modify

## Controls

| Input | Action |
|---|---|
| **WASD** | Move |
| **Mouse** | Aim |
| **Left click** | Punch / strike / shoot — on a downed rogue: finisher |
| **Right click** | Throw the held weapon |
| **E** | Pick up / swap the weapon you're standing on |
| **Shift** (hold) | Look ahead toward the cursor |
| **R** | After death: back to the last checkpoint. Held while alive: restart the floor |
| **Esc** | Pause (settings, about) |

`/?floor=N` starts directly on floor N; `&debug` enables the debug overlays (**I**): vision
cones, pathfinding waypoints, inflated wall bounds. Every URL flag:
[docs/URL_PARAMS.md](docs/URL_PARAMS.md).

## Building and Running

### Prerequisites

- Rust (install from [rustup.rs](https://rustup.rs/))
- Python 3 (dev server, level tooling)
- For the e2e tests: [Bun](https://bun.sh)

### Running Locally

The game is a `cdylib` (wasm) library — there is no native binary. The fastest
way to run it:

```bash
make build-wasm        # wasm32 build + wasm-bindgen glue (open_miami.js / open_miami_bg.wasm)
python3 serve.py       # dev server on :8080 (no-store caching, level-editor write API)
# then open http://localhost:8080  (?viz = tool panels, ?floor=N = start on floor N)
```

### Testing

```bash
make verify            # fmt, clippy, tests (incl. doc tests), release build, wasm build, level + prop data checks
make check-e2e         # browser e2e tests (Playwright on Bun) — see tests/e2e/README.md
make check-render      # renderer pixel-acceptance scripts (tests/e2e/render/), same toolchain
make verify-all        # verify + check-e2e + check-render
```

CI (`.github/workflows/`) runs exactly these Makefile targets, one job each.
What each suite covers, and where a new test belongs: [docs/TESTING.md](docs/TESTING.md).

### Building for the Web (WASM)

```bash
make build-wasm
```

This installs the `wasm32-unknown-unknown` target and `wasm-bindgen-cli` if
missing (the CLI is pinned to the `wasm-bindgen` version in `Cargo.lock` —
they must match exactly), builds the release wasm and generates the
JavaScript glue (`open_miami.js`, `open_miami_bg.wasm`). It is what
`make check-e2e` / `make check-render` and the CI build run.

Manually, the same steps are:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version <version of wasm-bindgen in Cargo.lock>
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen target/wasm32-unknown-unknown/release/open_miami.wasm \
    --out-dir . \
    --target web \
    --no-typescript
```

#### Running the Game

`python3 serve.py` (above) is the dev server: it disables caching and carries the level
editor's write API. To just PLAY a build, any static file server works
(`python3 -m http.server 8000`, then `http://localhost:8000`) — the game is `index.html`,
the generated `open_miami.js` / `open_miami_bg.wasm`, `web/` and `assets/`.

## Development

The Rust engine (a custom ECS, zero native dependencies) owns the simulation
and records each frame as a flat command stream; hand-written JS draws it
with WebGL. Four layers, one question each — the full picture is
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md):

```
open-miami/
├── src/                     # the Rust engine (compiles to wasm; most of it also natively, for tests)
│   ├── ecs/ components/ systems/   # SIM: the custom ECS, game data, gameplay systems
│   ├── scenario.rs game.rs sim.rs  #      scripted floors, spawning, the shared tick + headless Simulation
│   ├── render.rs render/           # RENDER: state -> draw commands (world, robots, comms, dialogue, …)
│   ├── props.rs props/             #         the prop library, split by family
│   ├── graphics.rs graphics/       #         the frame RECORDER (+ headless recording for tests)
│   ├── app.rs app/                 # APP (wasm-only): game loop, menus, the ?viz toolbox
│   ├── audio.rs audio/             # music as code + the WebAudio engine
│   └── editor.rs editor_ui.rs      # the native level editor
├── web/                     # RENDERER: hand-written JS / WebGL (renderer, shaders, opcode table,
│                            #           robot + boss 3D->2D pipelines)
├── levels/  props/          # floor + prop data (JSON) -> generated Rust (make gen-levels / gen-props)
├── tools/                   # ?viz panels, generators, the perf viewer
├── tests/                   # native integration tests + tests/e2e (Playwright specs, render scripts)
├── docs/                    # ARCHITECTURE, PIPELINE, TESTING, ECS, formats, URL params, music
├── index.html  serve.py     # the page and the dev server
└── Makefile                 # verify / build-wasm / check-* / bundle (CI calls these)
```

Start with [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) (where code goes and
why), then [docs/PIPELINE.md](docs/PIPELINE.md) (one frame, end to end),
[docs/ECS.md](docs/ECS.md) (the engine) and [docs/TESTING.md](docs/TESTING.md).
`CLAUDE.md` holds the working rules (what must never be broken, and why).

## Status and roadmap

The game is playable start to finish: gate, thirteen floors, the boss, 13½, the ending.
The engineering roadmap (layering, the characters moved from JS into Rust, the renderer's
measured GPU work) is done and recorded in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md);
its **"What's next"** section is the open-work list. Not planned yet: score tracking,
mobile touch controls.

## Technology

- **Rust -> WebAssembly** (`wasm-bindgen`, `web-sys`) - the simulation, the poses, the music
- **Hand-written WebGL** (`web/`, plain ES modules, no build step in dev) - the engine records
  each frame as one flat f32 command stream, handed to JS in a single zero-copy crossing
- **Custom everything** - ECS, A* + string pulling, level / prop formats, synth + sequencer
- **Tested on both sides of the boundary** - ~420 native tests run in seconds (every floor's
  real render path is recorded headlessly and validated), Playwright specs and pixel-parity
  scripts cover the browser ([docs/TESTING.md](docs/TESTING.md))

## Contributing

Contributions are welcome! Feel free to:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Inspiration

This project is inspired by [Hotline Miami](https://en.wikipedia.org/wiki/Hotline_Miami) by Dennaton Games. This is a fan project and is not affiliated with or endorsed by the original creators.

## Credits

Created by [c4ffein](https://github.com/c4ffein)

---

**Purge the rogues. Reach the elevator. EXFILTRATE.** 🎮
