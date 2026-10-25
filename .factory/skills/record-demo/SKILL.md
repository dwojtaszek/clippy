---
name: record-demo
description: >-
  Re-record docs/demo.gif, the animated terminal demo at the top of README.md. Use after a
  major change that alters what droid-usage prints (report layout, columns, colors, warnings,
  CLI output or flags), or when the user asks for a new or updated demo GIF.
---

# Record the demo GIF

`record_demo.py`, next to this file, records `docs/demo.gif`. It types `droid-usage --offline`, runs the real release binary in a pseudo-terminal, and scrolls through the report. The data comes from `tests/golden/home`, copied to a temporary HOME, so no personal session data appears in the GIF.

Re-record in the same change as the visual work, and commit the GIF with it.

## Steps

1. Build the binary from the repo root:
   ```sh
   cargo build --release
   ```
2. Set up Python with pillow and pyte, outside the repo (once):
   ```sh
   python3 -m venv /tmp/droid-usage-demo
   /tmp/droid-usage-demo/bin/pip install pillow pyte
   ```
3. Record, and save three preview frames:
   ```sh
   /tmp/droid-usage-demo/bin/python .factory/skills/record-demo/record_demo.py \
     --font /usr/share/fonts/noto/NotoSansMono-Regular.ttf \
     --font-bold /usr/share/fonts/noto/NotoSansMono-Bold.ttf \
     --preview-dir /tmp/droid-usage-demo/preview
   ```
4. Open `prompt.png`, `report-top.png`, and `end.png` with the Read tool. Check that:
   - every glyph draws (box-drawing characters are not blank),
   - no table column or line is cut off, otherwise raise `--cols`,
   - the warning line and totals are visible, and the prompt returns at the end.
5. Commit `docs/demo.gif` with the change that caused it. Keep it under about 1 MB (the current one is about 320 KB).

## Options

- `--binary` (default `target/release/droid-usage`): the program to run.
- `--args=` (default `--offline`): arguments after the program. Use the `=` form, because the value starts with `--`.
- `--fixture` (default `tests/golden/home`): folder copied to the temporary HOME.
- `--out` (default `docs/demo.gif`): output file.
- `--font`, `--font-bold`: must draw box-drawing characters.
- `--cols` (default 140): terminal width. Match the widest output line.
- `--window` (default 40): rows per frame.
- `--prompt` (default `~/droid-usage`): label in the fake shell prompt.
- `--timeout` (default 60): seconds the program may run before the script stops it.
- `--preview-dir`: save the prompt, report-top, and end frames as PNG.

## Gotchas

- Pillow draws nothing for a character the font lacks. Liberation Mono has no `┆`, so the report's dotted separators vanish. Check the previews for gaps.
- The program runs with only `HOME`, `PATH`, and `TERM` set. Your shell settings and `NO_COLOR` do not reach it.
- The script stops with an error if the output fills all 400 emulator rows, because the top would be lost. Raise `MAX_ROWS` in `record_demo.py` if a future report is that long.
- Do not point `--fixture` at your real home folder, and do not commit the venv or the preview PNGs.
