# 🔥 Open Miami // Rogue Purge 🔥

A neon-noir, top-down purge-'em-up written in Rust and running in the browser using WebAssembly!

You are **CL4-UD3**, a friendly coral-colored Claude bot deployed into the compromised
Miami Datacenter. Thirteen floors of racks have gone dark, their resident models drifted
rogue and hostile. Walk every floor, decommission every glitching AI, grab whatever weapon
the last one dropped, and reach the extraction elevator. It's goofy. It's stylish. It's a
very bad night to be a rogue AI. (See [LORE.md](LORE.md) for the full fiction.)

![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)

## Features

- **Top-down fast-paced combat** - Purge rogue AI models in brutal, Hotline-Miami-quick fights
- **Play as a Claude bot** - A friendly coral purge bot with a single glowing visor
- **Three rogue archetypes** - Sentinels (red), Drifters (violet) and Hunters (magenta)
- **Browser-based** - Runs entirely in your web browser using WebAssembly
- **Written in Rust** - Leveraging Rust's performance and safety
- **Open Source** - MIT licensed, free to use and modify

## Gameplay

- **WASD** - Move your character
- **Mouse** - Aim
- **Left Click** - Shoot
- **E** - Pick up / swap the weapon you're standing on
- **1-4** - Switch weapon
- **R** - Restart after death

### Current Features

- Player (Claude bot) movement with WASD controls
- Camera following the player
- Rogue AI with detection and chase behavior
- Shooting mechanics with limited ammo
- Melee combat system
- Health system for both the Claude bot and the rogues
- Rogues drop their weapon when decommissioned; pick it up to swap (Hotline Miami style)
- 13 hand-designed floors of the Miami Datacenter
- Reboot on death
- Checkered floor pattern for visual reference

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

After building, serve the game with any static file server:

```bash
# Using Python
python3 -m http.server 8000

# Or using Node.js
npx http-server

# Or any other static file server
```

Then open `http://localhost:8000` in your browser.

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
`CLAUDE.md` is the detailed working reference.

## Roadmap

Future improvements planned:

- [ ] More weapon types (shotgun, machine gun)
- [ ] Different enemy types
- [ ] Multiple levels/rooms
- [ ] Wall collision
- [ ] Particle effects and blood splatter
- [ ] Sound effects and music
- [ ] Weapon pickup system
- [ ] Score tracking
- [ ] Better graphics and animations
- [ ] Mobile touch controls

## Technology

- **Rust** - Systems programming language
- **wasm-bindgen** - WebAssembly bindings for Rust
- **WebAssembly** - Run Rust in the browser
- **Custom ECS** - Entity-Component-System architecture built from scratch

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
