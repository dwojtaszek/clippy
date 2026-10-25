#!/usr/bin/env python3
"""Record the demo GIF: type a command, run the real binary in a pseudo-terminal, scroll the output.

Needs pillow and pyte (see SKILL.md). The binary runs with HOME set to a temporary copy of the
fixture, so no personal session data reaches the recording.
"""

import argparse
import fcntl
import os
import pty
import select
import shlex
import shutil
import struct
import subprocess
import sys
import tempfile
import termios
import time

import pyte
from PIL import Image, ImageDraw, ImageFont

MAX_ROWS = 400  # rows the emulator keeps; output longer than this would lose its top
FONT_SIZE = 12
PAD = 12
BG = (13, 17, 23)
FG = (230, 237, 243)
DEFAULT_FONT = "/usr/share/fonts/noto/NotoSansMono-Regular.ttf"
DEFAULT_FONT_BOLD = "/usr/share/fonts/noto/NotoSansMono-Bold.ttf"
NAMED = {
    "black": "#484f58", "red": "#ff7b72", "green": "#3fb950", "brown": "#d29922",
    "yellow": "#d29922", "blue": "#58a6ff", "magenta": "#bc8cff", "cyan": "#39c5cf",
    "white": "#b1bac4", "brightblack": "#6e7681", "brightred": "#ffa198",
    "brightgreen": "#56d364", "brightbrown": "#e3b341", "brightyellow": "#e3b341",
    "brightblue": "#79c0ff", "brightmagenta": "#d2a8ff", "brightcyan": "#56d4dd",
    "brightwhite": "#f0f6fc",
}


def parse_args():
    parser = argparse.ArgumentParser(description="Record the demo GIF of a terminal program.")
    parser.add_argument("--binary", default="target/release/droid-usage")
    parser.add_argument(
        "--args", dest="binary_args", default="--offline",
        help="arguments after the program, as one string; write it as --args=--offline",
    )
    parser.add_argument("--fixture", default="tests/golden/home", help="folder copied to a temporary HOME")
    parser.add_argument("--out", default="docs/demo.gif")
    parser.add_argument("--font", default=DEFAULT_FONT, help="monospace font that draws box-drawing characters")
    parser.add_argument("--font-bold", default=DEFAULT_FONT_BOLD)
    parser.add_argument("--cols", type=int, default=140, help="terminal width; must fit the widest output line")
    parser.add_argument("--window", type=int, default=40, help="rows shown in each frame")
    parser.add_argument("--prompt", default="~/droid-usage", help="label in the fake shell prompt")
    parser.add_argument("--timeout", type=float, default=60, help="seconds the program may run")
    parser.add_argument("--preview-dir", help="also save the prompt, report-top, and end frames here as PNG")
    return parser.parse_args()


def to_rgb(name, default):
    if name == "default":
        return default
    if name in NAMED:
        value = NAMED[name]
    elif len(name) == 6 and all(c in "0123456789abcdefABCDEF" for c in name):
        value = "#" + name
    else:
        return default
    return tuple(int(value[i:i + 2], 16) for i in (1, 3, 5))


def run_binary(cmd, env, stream, cols, timeout):
    """Run cmd in a pseudo-terminal and feed everything it prints to the emulator."""
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 60, cols, 0, 0))
    proc = subprocess.Popen(cmd, stdin=slave, stdout=slave, stderr=slave, env=env)
    os.close(slave)
    deadline = time.monotonic() + timeout
    try:
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                proc.kill()
                sys.exit(f"{cmd[0]} ran longer than {timeout:g} s")
            ready, _, _ = select.select([master], [], [], remaining)
            if not ready:
                continue
            try:
                data = os.read(master, 65536)
            except OSError:
                break  # the child closed its end of the terminal
            if not data:
                break
            stream.feed(data)
    finally:
        os.close(master)
    exit_code = proc.wait()
    if exit_code != 0:
        sys.exit(f"{cmd[0]} exited with {exit_code}")


def main():
    args = parse_args()
    binary = os.path.abspath(args.binary)
    fixture = os.path.abspath(args.fixture)
    if not os.path.isfile(binary):
        sys.exit(f"no program at {binary}; run cargo build --release first")
    if not os.path.isdir(fixture):
        sys.exit(f"no fixture folder at {fixture}")

    font = ImageFont.truetype(args.font, FONT_SIZE)
    bold = ImageFont.truetype(args.font_bold, FONT_SIZE)
    cell_w = round(font.getlength("M"))
    ascent, descent = font.getmetrics()
    cell_h = ascent + descent
    width = 2 * PAD + args.cols * cell_w
    height = 2 * PAD + args.window * cell_h

    screen = pyte.Screen(args.cols, MAX_ROWS)
    stream = pyte.ByteStream(screen)
    prompt = f"\x1b[1;32m{args.prompt}\x1b[0m \x1b[1;34m$\x1b[0m ".encode()
    frames = []  # (image, duration in ms)

    def render(top, show_cursor):
        img = Image.new("RGB", (width, height), BG)
        draw = ImageDraw.Draw(img)
        for row in range(args.window):
            line = screen.buffer[top + row]
            y = PAD + row * cell_h
            for col in range(args.cols):
                ch = line[col]
                x = PAD + col * cell_w
                fg = to_rgb(ch.fg, FG)
                bg = to_rgb(ch.bg, BG)
                if ch.reverse:
                    fg, bg = bg, fg
                is_cursor = show_cursor and top + row == screen.cursor.y and col == screen.cursor.x
                if is_cursor:
                    fg, bg = BG, FG
                if bg != BG:
                    draw.rectangle([x, y, x + cell_w - 1, y + cell_h - 1], fill=bg)
                if ch.data.strip():
                    draw.text((x, y), ch.data, font=bold if ch.bold else font, fill=fg)
        return img

    def snap(ms, top=0, show_cursor=False):
        frames.append((render(top, show_cursor), ms))

    with tempfile.TemporaryDirectory() as tmp:
        home = os.path.join(tmp, "home")
        shutil.copytree(fixture, home)
        env = {"HOME": home, "PATH": os.environ["PATH"], "TERM": "xterm-256color"}

        # 1. Shell prompt, then the command typed one key at a time.
        typed = f"{os.path.basename(binary)} {args.binary_args}"
        stream.feed(prompt)
        snap(700, show_cursor=True)
        for ch in typed:
            stream.feed(ch.encode())
            snap(85, show_cursor=True)
        stream.feed(b"\r\n")
        snap(500)

        # 2. Run the real program in a pseudo-terminal, so its colors and table widths match a terminal.
        run_binary([binary, *shlex.split(args.binary_args)], env, stream, args.cols, args.timeout)

    if screen.cursor.y >= MAX_ROWS - 1:
        sys.exit(f"output filled the {MAX_ROWS} emulator rows; raise MAX_ROWS in {os.path.basename(__file__)}")
    stream.feed(prompt)
    total = screen.cursor.y + 1
    last_top = max(0, total - args.window)

    # 3. Hold the top of the output, scroll down through it, then hold the end.
    top_index = len(frames)
    snap(1800, top=0)
    step = 8
    tops = list(range(step, last_top, step)) + [last_top]
    for top in tops:
        snap(350, top=top, show_cursor=(top == last_top))
    frames[-1] = (frames[-1][0], 4000)

    if args.preview_dir:
        os.makedirs(args.preview_dir, exist_ok=True)
        for name, index in (("prompt", 0), ("report-top", top_index), ("end", len(frames) - 1)):
            frames[index][0].save(os.path.join(args.preview_dir, f"{name}.png"))

    out = os.path.abspath(args.out)
    os.makedirs(os.path.dirname(out), exist_ok=True)
    images = [f[0] for f in frames]
    durations = [f[1] for f in frames]
    images[0].save(
        out,
        save_all=True,
        append_images=images[1:],
        duration=durations,
        loop=0,
        optimize=True,
    )
    print(
        f"wrote {args.out}: {len(frames)} frames, {sum(durations) / 1000:.1f} s, "
        f"{width}x{height}, {os.path.getsize(out) / 1024:.0f} KiB"
    )


if __name__ == "__main__":
    main()
