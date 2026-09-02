#!/usr/bin/env python3
"""Generate `src/audio/songs_data.rs` from `songs/*.json`.

Python standard library only (no third-party modules, no Rust crates on the
other side: the output is plain `const` data using `&'static str` /
`&'static [..]` slices that `src/audio/songs.rs` types describe).

Usage:
    python3 tools/gen_songs.py            # write src/audio/songs_data.rs
    python3 tools/gen_songs.py --check    # validate + verify the checked-in
                                          # file is up to date (exit 1 if not)

The JSON contract is documented in docs/SONGS_FORMAT.md. This script also
validates it: every lane token must be a rest or an integer scale degree
(drum lanes: `.`, `k`, `h`, `s`), every `@pattern` reference must resolve,
every section and pattern must be used, the arrangement `order` must only
name existing sections, every section's longest lane must be whole bars
(the sequencer advances sections when the LONGEST lane ends), waves and
named scales must be from the fixed sets, and no two songs may share a name
or a file.
"""
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SONGS_DIR = os.path.join(ROOT, "songs")
OUT_PATH = os.path.join(ROOT, "src", "audio", "songs_data.rs")

# Oscillator shapes (mirrors `Wave` in src/audio/songs.rs).
WAVES = {"sine": "Sine", "square": "Square", "sawtooth": "Sawtooth", "triangle": "Triangle"}
# Named scales (mirrors the `Scale` consts in src/audio/songs.rs).
SCALES = {
    "minor": "MINOR",
    "dorian": "DORIAN",
    "harmonic_minor": "HARMONIC_MINOR",
    "phrygian": "PHRYGIAN",
    "phrygian_dominant": "PHRYGIAN_DOMINANT",
    "locrian": "LOCRIAN",
}
# One character per drum step (mirrors `Drum` in src/audio/songs.rs).
DRUMS = {".": "Silent", "k": "Kick", "h": "Hat", "s": "Snare"}
MELODIC_LANES = ("bass", "lead", "pad", "arp")
LANES = MELODIC_LANES + ("drums",)
IDENT = re.compile(r"^[a-z][a-z0-9_]*$")
# Rust `i32` range for a scale degree — anything else is a typo.
DEGREE_RANGE = (-64, 64)


class Invalid(Exception):
    pass


def num(v, what, positive=False):
    """A finite JSON number, formatted as a Rust f64 literal, deterministically."""
    if isinstance(v, bool) or not isinstance(v, (int, float)):
        raise Invalid(f"{what}: expected a number, got {v!r}")
    v = float(v)
    if v != v or v in (float("inf"), float("-inf")):
        raise Invalid(f"{what}: non-finite number")
    if positive and v <= 0:
        raise Invalid(f"{what}: must be > 0")
    if v == int(v) and abs(v) < 1e15:
        return f"{int(v)}.0"
    return repr(v)


def rstr(s, what):
    """A Rust string literal (UTF-8 passthrough, escaping only what must be)."""
    if not isinstance(s, str):
        raise Invalid(f"{what}: expected a string, got {s!r}")
    out = s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n").replace("\r", "").replace("\t", "\\t")
    return f'"{out}"'


def ident(name, what):
    if not isinstance(name, str) or not IDENT.match(name):
        raise Invalid(f"{what}: {name!r} is not a snake_case identifier ([a-z][a-z0-9_]*)")
    return name.upper()


def lane_text(v, what):
    """A lane's token source: one string, or a list of strings (one per bar,
    joined). `|` is a purely visual bar separator."""
    if isinstance(v, str):
        parts = [v]
    elif isinstance(v, list) and v and all(isinstance(p, str) for p in v):
        parts = v
    else:
        raise Invalid(f"{what}: expected a token string or a non-empty list of them, got {v!r}")
    return " ".join(p.replace("|", " ") for p in parts)


def parse_melodic(v, what):
    """`.` = rest, an integer = a scale degree; whitespace-separated."""
    out = []
    for tok in lane_text(v, what).split():
        if tok == ".":
            out.append("REST")
            continue
        try:
            d = int(tok, 10)
        except ValueError:
            raise Invalid(f"{what}: bad token {tok!r} (expected '.' or an integer degree)")
        if not DEGREE_RANGE[0] <= d <= DEGREE_RANGE[1]:
            raise Invalid(f"{what}: degree {d} out of range {DEGREE_RANGE}")
        out.append(str(d))
    return out


def parse_drums(v, what):
    """One character per step: `.` silent, `k` kick, `h` hat, `s` snare;
    whitespace is ignored."""
    out = []
    for ch in "".join(lane_text(v, what).split()):
        if ch not in DRUMS:
            raise Invalid(f"{what}: bad drum step {ch!r} (expected one of {''.join(DRUMS)})")
        out.append(DRUMS[ch])
    return out


def load_songs():
    with open(os.path.join(SONGS_DIR, "index.json"), encoding="utf-8") as fh:
        index = json.load(fh)
    if not isinstance(index.get("songs"), list) or not index["songs"]:
        raise Invalid("index.json: 'songs' must be a non-empty list")
    songs = []
    seen_files = set()
    for entry in index["songs"]:
        if not isinstance(entry, dict) or "file" not in entry:
            raise Invalid(f"index.json: bad entry {entry!r}")
        file = entry["file"]
        if file in seen_files:
            raise Invalid(f"index.json: {file} listed twice")
        seen_files.add(file)
        stem, ext = os.path.splitext(file)
        if ext != ".json" or not IDENT.match(stem):
            raise Invalid(f"index.json: {file!r} must be <snake_case>.json")
        with open(os.path.join(SONGS_DIR, file), encoding="utf-8") as fh:
            song = json.load(fh)
        if not isinstance(song, dict):
            raise Invalid(f"{file}: expected an object")
        if "name" in entry and song.get("name") != entry["name"]:
            raise Invalid(f"{file}: name {song.get('name')!r} != index name {entry['name']!r}")
        song["_file"] = file
        song["_ident"] = stem.upper()
        songs.append(song)
    return songs


def validate_song(s):
    """Validate one song and return its compiled form (ready to emit)."""
    what = s["_file"]
    for key in ("name", "root", "scale", "bpm", "steps_per_beat", "waves", "intensity", "sections", "order"):
        if key not in s:
            raise Invalid(f"{what}: missing '{key}'")
    known = {"name", "notes", "root", "root_note", "scale", "bpm", "steps_per_beat", "waves",
             "intensity", "patterns", "sections", "order", "_file", "_ident"}
    for key in s:
        if key not in known:
            raise Invalid(f"{what}: unknown key '{key}'")
    name = rstr(s["name"], f"{what}: name")
    if not s["name"]:
        raise Invalid(f"{what}: empty name")
    notes = s.get("notes", [])
    if not isinstance(notes, list) or not all(isinstance(n, str) for n in notes):
        raise Invalid(f"{what}: 'notes' must be a list of strings")
    if "root_note" in s and not isinstance(s["root_note"], str):
        raise Invalid(f"{what}: 'root_note' must be a string")
    root = num(s["root"], f"{what}: root", positive=True)
    bpm = num(s["bpm"], f"{what}: bpm", positive=True)
    spb = s["steps_per_beat"]
    if isinstance(spb, bool) or not isinstance(spb, int) or spb < 1:
        raise Invalid(f"{what}: steps_per_beat must be an integer >= 1")
    bar = 4 * spb
    intensity = num(s["intensity"], f"{what}: intensity")
    scale = s["scale"]
    if isinstance(scale, str):
        if scale not in SCALES:
            raise Invalid(f"{what}: unknown scale {scale!r} (known: {', '.join(SCALES)})")
        scale_rs = SCALES[scale]
    elif isinstance(scale, list) and scale and all(
        isinstance(x, int) and not isinstance(x, bool) and 0 <= x < 12 for x in scale
    ):
        scale_rs = "&[" + ", ".join(str(x) for x in scale) + "]"
    else:
        raise Invalid(f"{what}: 'scale' must be a scale name or a non-empty list of semitones 0..11")
    waves = s["waves"]
    if not isinstance(waves, dict) or set(waves) != set(MELODIC_LANES):
        raise Invalid(f"{what}: 'waves' must have exactly the keys {', '.join(MELODIC_LANES)}")
    waves_rs = {}
    for lane in MELODIC_LANES:
        if waves[lane] not in WAVES:
            raise Invalid(f"{what}: waves.{lane}: unknown wave {waves[lane]!r} (known: {', '.join(WAVES)})")
        waves_rs[lane] = "Wave::" + WAVES[waves[lane]]

    # Named patterns: parsed lazily by the lane kind that references them
    # (a pattern is melodic or drums depending on where it is used).
    patterns = s.get("patterns", {})
    if not isinstance(patterns, dict):
        raise Invalid(f"{what}: 'patterns' must be an object")
    pattern_rs = {}  # name -> (kind, steps)
    pattern_used = set()

    def resolve(lane, v, where):
        parse = parse_drums if lane == "drums" else parse_melodic
        kind = "drums" if lane == "drums" else "melodic"
        if isinstance(v, str) and v.startswith("@"):
            pname = v[1:]
            if pname not in patterns:
                raise Invalid(f"{where}: unknown pattern {v!r}")
            ident(pname, f"{what}: patterns.{pname}")
            pattern_used.add(pname)
            if pname in pattern_rs:
                if pattern_rs[pname][0] != kind:
                    raise Invalid(f"{where}: pattern {v!r} is used as both a drum and a melodic lane")
            else:
                pattern_rs[pname] = (kind, parse(patterns[pname], f"{what}: patterns.{pname}"))
            return ("ref", pname, pattern_rs[pname][1])
        return ("inline", None, parse(v, where))

    sections = s["sections"]
    if not isinstance(sections, dict) or not sections:
        raise Invalid(f"{what}: 'sections' must be a non-empty object")
    sections_rs = {}
    for key, sec in sections.items():
        swhat = f"{what}: sections.{key}"
        ident(key, swhat)
        if not isinstance(sec, dict):
            raise Invalid(f"{swhat}: expected an object")
        for k in sec:
            if k not in LANES and k != "label":
                raise Invalid(f"{swhat}: unknown key '{k}'")
        for lane in LANES:
            if lane not in sec:
                raise Invalid(f"{swhat}: missing lane '{lane}'")
        label = sec.get("label", key)
        lanes = {lane: resolve(lane, sec[lane], f"{swhat}.{lane}") for lane in LANES}
        longest = max(max(len(v[2]) for v in lanes.values()), 1)
        if longest % bar != 0:
            raise Invalid(f"{swhat}: longest lane is {longest} steps — not whole bars of {bar}")
        sections_rs[key] = (rstr(label, f"{swhat}.label"), lanes)
    order = s["order"]
    if not isinstance(order, list) or not order:
        raise Invalid(f"{what}: 'order' must be a non-empty list of section keys")
    for key in order:
        if key not in sections:
            raise Invalid(f"{what}: order names unknown section {key!r}")
    unused = sorted(set(sections) - set(order))
    if unused:
        raise Invalid(f"{what}: sections never played: {', '.join(unused)}")
    unused = sorted(set(patterns) - pattern_used)
    if unused:
        raise Invalid(f"{what}: patterns never used: {', '.join(unused)}")
    return {
        "file": what,
        "ident": s["_ident"],
        "name": name,
        "raw_name": s["name"],
        "notes": notes,
        "root": root,
        "root_note": s.get("root_note"),
        "scale": scale_rs,
        "bpm": bpm,
        "steps_per_beat": spb,
        "bar": bar,
        "waves": waves_rs,
        "intensity": intensity,
        "patterns": pattern_rs,
        "sections": sections_rs,
        "order": order,
    }


def validate(songs):
    compiled = [validate_song(s) for s in songs]
    names = set()
    for c in compiled:
        if c["raw_name"] in names:
            raise Invalid(f"{c['file']}: duplicate song name {c['raw_name']!r}")
        names.add(c["raw_name"])
    return compiled


def lane_lines(steps, bar, out, indent="        "):
    """Emit `&[..]` — one bar per line, mirroring the hand-written layout."""
    if not steps:
        out.append("&[]")
        return
    if len(steps) <= bar and len(", ".join(steps)) + len(indent) + 4 <= 100:
        out.append("&[" + ", ".join(steps) + "]")
        return
    out.append("&[")
    for i in range(0, len(steps), bar):
        out.append(indent + ", ".join(steps[i:i + bar]) + ",")
    out.append(indent[:-4] + "]")


def gen_song(c, out):
    ident_ = c["ident"]
    out.append(f"// ---- {c['file']}: {c['name']} " + "-" * max(4, 88 - len(c["file"]) - len(c["name"])))
    if c["notes"]:
        out.append("//")
        for line in c["notes"]:
            out.append(("// " + line).rstrip())
    out.append("")
    for pname, (_kind, steps) in c["patterns"].items():
        parts = []
        lane_lines(steps, c["bar"], parts, indent="    ")
        parts[0] = f"const {ident_}_PAT_{pname.upper()}: &[{'Drum' if _kind == 'drums' else 'i32'}] = " + parts[0]
        parts[-1] += ";"
        out.extend(parts)
    section_idents = {}
    for key, (label, lanes) in c["sections"].items():
        sid = f"{ident_}_{key.upper()}"
        section_idents[key] = sid
        out.append(f"const {sid}: Section = Section {{")
        out.append(f"    label: {label},")
        for lane in LANES:
            kind, pname, steps = lanes[lane]
            if kind == "ref":
                out.append(f"    {lane}: {ident_}_PAT_{pname.upper()},")
                continue
            parts = []
            lane_lines(steps, c["bar"], parts)
            parts[0] = f"    {lane}: " + parts[0]
            parts[-1] += ","
            out.extend(parts)
        out.append("};")
    out.append(f"pub const {ident_}: SongSpec = SongSpec {{")
    out.append(f"    name: {c['name']},")
    root_note = f" // {c['root_note']}" if c["root_note"] else ""
    out.append(f"    root: {c['root']},{root_note}")
    out.append(f"    scale: {c['scale']},")
    out.append(f"    bpm: {c['bpm']},")
    out.append(f"    steps_per_beat: {c['steps_per_beat']},")
    for lane in MELODIC_LANES:
        out.append(f"    {lane}_wave: {c['waves'][lane]},")
    out.append("    sections: &[")
    for key in c["order"]:
        out.append(f"        {section_idents[key]},")
    out.append("    ],")
    out.append(f"    intensity: {c['intensity']},")
    out.append("};")
    out.append("")
    return ident_


def generate(compiled):
    out = [
        "// @generated by tools/gen_songs.py from songs/*.json — DO NOT EDIT.",
        "// Re-run `make gen-songs` after editing the JSON.",
        "//",
        "// Songs in `SONGS` order (songs/index.json); see docs/SONGS_FORMAT.md for the",
        "// contract and src/audio/songs.rs for the types.",
        "#![allow(clippy::all)]",
        "#![allow(clippy::excessive_precision)]",
        "// Not every song set uses every scale / drum the types offer.",
        "#![allow(unused_imports)]",
        "",
        "use super::songs::{",
        "    Drum::{self, Hat, Kick, Silent, Snare},",
        "    Scale, Section, SongSpec, Wave, DORIAN, HARMONIC_MINOR, LOCRIAN, MINOR, PHRYGIAN,",
        "    PHRYGIAN_DOMINANT, REST,",
        "};",
        "",
    ]
    idents = [gen_song(c, out) for c in compiled]
    out.append("/// Number of songs.")
    out.append(f"pub const SONG_COUNT: usize = {len(idents)};")
    out.append("")
    out.append("/// Every song, in songs/index.json order (the `?viz` tracker's list; floors")
    out.append("/// pick theirs through `song_for_floor`).")
    out.append("pub const SONGS: &[SongSpec; SONG_COUNT] = &[")
    for i in idents:
        out.append(f"    {i},")
    out.append("];")
    out.append("")
    return "\n".join(out)


def main(argv):
    check = "--check" in argv
    try:
        songs = load_songs()
        compiled = validate(songs)
        text = generate(compiled)
    except (Invalid, KeyError, OSError, ValueError, json.JSONDecodeError) as e:
        print(f"gen_songs: error: {e}", file=sys.stderr)
        return 1
    if check:
        try:
            with open(OUT_PATH, encoding="utf-8") as fh:
                current = fh.read()
        except OSError:
            current = None
        if current != text:
            print(f"gen_songs: {os.path.relpath(OUT_PATH, ROOT)} is out of date — run `make gen-songs`",
                  file=sys.stderr)
            return 1
        print(f"gen_songs: {len(songs)} songs valid, {os.path.relpath(OUT_PATH, ROOT)} up to date")
        return 0
    with open(OUT_PATH, "w", encoding="utf-8") as fh:
        fh.write(text)
    print(f"gen_songs: wrote {os.path.relpath(OUT_PATH, ROOT)} ({len(songs)} songs)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
