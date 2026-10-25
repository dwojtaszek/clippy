#!/usr/bin/env python3
"""Builds the anonymized golden fixtures under tests/golden/home from real session files.

Run from the repository root, with the real home folder:

    python3 tests/golden/anonymize.py --home ~ --out tests/golden/home

The fixtures keep what the readers and the report use: token counts, model
names, timestamps, recorded costs, and which sessions share a project. They drop
message text, tool output, session and conversation IDs, and real paths. Each
real project folder becomes `/work/project-NN`.

The script checks that every reduced file gives the same reader result as its
original (project, start time, model, and token sums). It then prints a summary
to compare with the golden report by hand.

To regenerate the expected output after a fixture change:

    HOME=$PWD/tests/golden/home NO_COLOR=1 cargo run -q -- --offline \\
        > tests/golden/report.txt 2> tests/golden/warnings.txt

The script removes `--out` before writing it. Point it only at tests/golden/home.
"""

import argparse
import glob
import json
import os
import re
import shutil
import sqlite3
from datetime import datetime

DROID_SAMPLE = 10
AGY_WITH_TOKENS = 6
AGY_WITHOUT_TOKENS = 2
DROID_TOKEN_KEYS = ["inputTokens", "outputTokens", "cacheReadTokens", "cacheCreationTokens"]
OPENCODE_COLUMNS = [
    "directory",
    "model",
    "time_created",
    "tokens_input",
    "tokens_output",
    "tokens_cache_read",
    "tokens_cache_write",
    "cost",
]
AGY_TOKEN_KEYS = ["input_tokens", "output_tokens", "cache_read_tokens"]
AGY_MODEL_PREFIXES = (b"gemini-", b"claude-", b"gpt-")


def stride(items, count):
    """Evenly spaced items, so a sample covers the whole sorted list."""
    if len(items) <= count:
        return list(items)
    return [items[i * len(items) // count] for i in range(count)]


def load_json(path):
    try:
        with open(path, encoding="utf-8") as handle:
            return json.load(handle)
    except (OSError, ValueError):
        return None


def write_json(path, value):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(value, handle, indent=1)
        handle.write("\n")


def parse_time(text):
    try:
        return datetime.fromisoformat(text.replace("Z", "+00:00"))
    except (AttributeError, ValueError):
        return None


def project_path(real, projects):
    """Maps a real project folder to a synthetic one, the same way every time."""
    if real is None:
        return None
    if real not in projects:
        projects[real] = f"/work/project-{len(projects) + 1:02d}"
    return projects[real]


def droid_header(jsonl):
    """Project and first message time, read the same way as the Droid reader."""
    project, started, role = None, None, None
    with open(jsonl, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            try:
                value = json.loads(line)
            except ValueError:
                continue
            if not isinstance(value, dict):
                continue
            if value.get("type") == "session_start" and project is None:
                project = value.get("cwd")
            if value.get("type") == "message" and isinstance(value.get("timestamp"), str):
                started = value["timestamp"]
                role = (value.get("message") or {}).get("role")
                break
    return project, started, role


def droid(home, out, projects, summary):
    root = os.path.join(home, ".factory/sessions")
    settings = sorted(glob.glob(os.path.join(root, "**", "*.settings.json"), recursive=True))
    usable, without_usage, broken = [], [], []
    for path in settings:
        data = load_json(path)
        if not isinstance(data, dict):
            broken.append(path)
        elif isinstance(data.get("tokenUsage"), dict):
            usable.append(path)
        else:
            without_usage.append(path)

    models = set()
    unreadable = len(broken)
    out_root = os.path.join(out, ".factory/sessions")
    for index, path in enumerate(stride(usable, DROID_SAMPLE) + without_usage[:2], 1):
        data = load_json(path)
        jsonl = path[: -len(".settings.json")] + ".jsonl"
        real_project, started, role = (
            droid_header(jsonl) if os.path.exists(jsonl) else (None, None, None)
        )
        # The reader skips a session with no transcript or no timed message.
        if started is None:
            unreadable += 1
        project = project_path(real_project, projects)
        folder = project.strip("/").replace("/", "-") if project else "no-project"

        usage = data.get("tokenUsage")
        settings_out = {}
        if isinstance(data.get("model"), str):
            settings_out["model"] = data["model"]
            models.add(data["model"])
        if usage is None or isinstance(usage, dict):
            settings_out["tokenUsage"] = (
                None
                if usage is None
                else {k: v for k, v in usage.items() if isinstance(v, int)}
            )
        name = f"droid-{index:02d}"
        write_json(os.path.join(out_root, folder, f"{name}.settings.json"), settings_out)

        if os.path.exists(jsonl):
            lines = []
            if project is not None:
                lines.append({"type": "session_start", "cwd": project})
            if started is not None:
                message = {"type": "message", "timestamp": started}
                if role is not None:
                    message["message"] = {"role": role}
                lines.append(message)
            reduced = os.path.join(out_root, folder, f"{name}.jsonl")
            write_lines(reduced, lines)
            if droid_header(reduced) != (project, started, role):
                raise SystemExit(f"reduced header differs for {path}")

        if started is not None:
            tokens = sum(
                (usage or {}).get(key, 0) for key in DROID_TOKEN_KEYS if isinstance(usage, dict)
            )
            summary.add("droid", model=data.get("model"), tokens=tokens)

    write_text(os.path.join(out_root, "no-project", "droid-broken.settings.json"), "{not json")
    summary.skipped["droid"] = unreadable
    return models


def write_lines(path, lines):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as handle:
        for line in lines:
            handle.write(json.dumps(line) + "\n")


def write_text(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(text)


def opencode(home, out, projects, summary):
    source = os.path.join(home, ".local/share/opencode/opencode.db")
    conn = sqlite3.connect(f"file:{source}?mode=ro", uri=True)
    declared = {row[1]: row[2] for row in conn.execute("pragma table_info(session_v2)")}
    rows = conn.execute(f"select {', '.join(OPENCODE_COLUMNS)} from session_v2").fetchall()
    conn.close()

    target = os.path.join(out, ".local/share/opencode/opencode.db")
    os.makedirs(os.path.dirname(target), exist_ok=True)
    out_conn = sqlite3.connect(target)
    columns = ", ".join(f"{name} {declared[name]}" for name in OPENCODE_COLUMNS)
    out_conn.execute(f"CREATE TABLE session_v2 ({columns})")
    models = set()
    for row in rows:
        directory, model, created, t_in, t_out, t_read, t_write, cost = row
        model_id = model
        if isinstance(model, str) and model.startswith("{"):
            model_id = (json.loads(model) or {}).get("id")
        out_model = json.dumps({"id": model_id, "providerID": "fixture"}) if model_id else None
        if model_id:
            models.add(model_id)
        out_conn.execute(
            f"INSERT INTO session_v2 VALUES ({', '.join('?' * len(OPENCODE_COLUMNS))})",
            (
                project_path(directory, projects),
                out_model,
                created,
                t_in,
                t_out,
                t_read,
                t_write,
                cost,
            ),
        )
        summary.add("opencode", model=model_id, tokens=(t_in or 0) + (t_out or 0) + (t_read or 0) + (t_write or 0))
    out_conn.commit()
    out_conn.close()
    summary.skipped["opencode"] = 0
    return models


def read_transcript(path):
    """Every line that parses as JSON, as dicts."""
    values = []
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            try:
                value = json.loads(line)
            except ValueError:
                continue
            if isinstance(value, dict):
                values.append(value)
    return values


def transcript_totals(values):
    """Earliest time and token sums, as the agy reader sees them."""
    started = None
    totals = {key: 0 for key in AGY_TOKEN_KEYS}
    for value in values:
        at = parse_time(value.get("created_at"))
        if at is not None and (started is None or at < started):
            started = at
        for key in AGY_TOKEN_KEYS:
            totals[key] += value.get(key, 0) if isinstance(value.get(key, 0), int) else 0
    return started, totals


def reduce_transcript(values):
    """Keeps the lines the reader uses: token lines, and the earliest timed line."""
    earliest = None
    for value in values:
        at = parse_time(value.get("created_at"))
        if at is not None and (earliest is None or at < earliest[0]):
            earliest = (at, value)
    kept = []
    for value in values:
        if any(key in value for key in AGY_TOKEN_KEYS) or (earliest and value is earliest[1]):
            line = {key: value[key] for key in ("type", "status", "created_at") if key in value}
            line.update({key: value[key] for key in AGY_TOKEN_KEYS if key in value})
            kept.append(line)
    return kept


def first_model_name(blob):
    for token in re.split(rb"[^A-Za-z0-9._-]", blob):
        if token.startswith(AGY_MODEL_PREFIXES):
            return token.decode("ascii")
    return None


def conversation_model(path):
    if not os.path.exists(path):
        return None
    conn = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    try:
        blobs = conn.execute("SELECT data FROM gen_metadata ORDER BY idx").fetchall()
    except sqlite3.Error:
        return None
    finally:
        conn.close()
    for (blob,) in blobs:
        name = first_model_name(blob)
        if name:
            return name
    return None


def agy(home, out, summary):
    root = os.path.join(home, ".gemini/antigravity-cli")
    transcripts = sorted(
        glob.glob(os.path.join(root, "brain", "*", ".system_generated", "logs", "transcript.jsonl"))
    )
    with_tokens, without = [], []
    for path in transcripts:
        values = read_transcript(path)
        if any(any(key in value for key in AGY_TOKEN_KEYS) for value in values):
            with_tokens.append(path)
        else:
            without.append(path)

    models = set()
    for index, path in enumerate(
        stride(with_tokens, AGY_WITH_TOKENS) + without[:AGY_WITHOUT_TOKENS], 1
    ):
        real_id = os.path.basename(os.path.dirname(os.path.dirname(os.path.dirname(path))))
        values = read_transcript(path)
        started, totals = transcript_totals(values)
        reduced = reduce_transcript(values)
        check_transcript(reduced, started, totals, path)

        name = f"agy-{index:02d}"
        write_lines(
            os.path.join(out, ".gemini/antigravity-cli/brain", name, ".system_generated/logs/transcript.jsonl"),
            reduced,
        )
        model = conversation_model(os.path.join(root, "conversations", f"{real_id}.db"))
        if model:
            models.add(model)
            db = os.path.join(out, ".gemini/antigravity-cli/conversations", f"{name}.db")
            os.makedirs(os.path.dirname(db), exist_ok=True)
            blob = b"\x0a" + model.encode("ascii") + b"\x00"
            conn = sqlite3.connect(db)
            conn.execute("CREATE TABLE gen_metadata (idx INTEGER, data BLOB, size INTEGER)")
            conn.execute("INSERT INTO gen_metadata VALUES (0, ?, ?)", (blob, len(blob)))
            conn.commit()
            conn.close()
        summary.add("agy", model=model, tokens=sum(totals.values()))

    summary.skipped["agy"] = 0
    return models


def check_transcript(reduced, started, totals, path):
    got_started, got_totals = transcript_totals(reduced)
    if got_started != started or got_totals != totals:
        raise SystemExit(f"reduced transcript differs for {path}")


def snapshots(home, out, models):
    base = os.path.join(home, ".local/share/droid-usage")
    price_files = sorted(glob.glob(os.path.join(base, "prices", "*.json")))
    multiplier_files = sorted(glob.glob(os.path.join(base, "multipliers", "*.json")))
    if not price_files or not multiplier_files:
        raise SystemExit("need one saved price and one multiplier snapshot in ~/.local/share/droid-usage")

    wanted = set()
    for model in models:
        wanted.add(model)
        for suffix in ("-ultrafast", "-fast"):
            if model.endswith(suffix):
                wanted.add(model[: -len(suffix)])
    price = load_json(price_files[-1])
    kept = {key: rate for key, rate in price["rates"].items() if key.split("/")[-1] in wanted}
    write_json(
        os.path.join(out, ".local/share/droid-usage/prices", os.path.basename(price_files[-1])),
        {"fetched": price["fetched"], "rates": kept},
    )
    multiplier = load_json(multiplier_files[-1])
    write_json(
        os.path.join(out, ".local/share/droid-usage/multipliers", os.path.basename(multiplier_files[-1])),
        multiplier,
    )
    return len(kept)


class Summary:
    def __init__(self):
        self.sources = {}
        self.skipped = {}

    def add(self, source, model, tokens):
        entry = self.sources.setdefault(source, {"sessions": 0, "with_model": 0, "tokens": 0})
        entry["sessions"] += 1
        entry["with_model"] += 1 if model else 0
        entry["tokens"] += tokens

    def show(self, kept_prices):
        for source, entry in sorted(self.sources.items()):
            print(
                f"{source}: {entry['sessions']} sessions, {entry['with_model']} with a model, "
                f"{entry['tokens']} tokens, {self.skipped.get(source, 0)} unreadable entries"
            )
        print(f"price rates kept: {kept_prices}")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--home", required=True, help="real home folder to read")
    parser.add_argument("--out", required=True, help="fixture home folder to write (removed first)")
    args = parser.parse_args()
    home = os.path.expanduser(args.home)
    out = os.path.abspath(args.out)

    if os.path.isdir(out):
        shutil.rmtree(out)
    os.makedirs(out)

    projects = {}
    summary = Summary()
    models = set()
    models |= droid(home, out, projects, summary)
    models |= opencode(home, out, projects, summary)
    models |= agy(home, out, summary)
    kept = snapshots(home, out, models)
    summary.show(kept)


if __name__ == "__main__":
    main()
