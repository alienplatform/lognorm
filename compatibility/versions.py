#!/usr/bin/env python3
"""Run selected logger versions in isolated environments without refreshing fixtures."""

import argparse
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
PROFILES = json.loads((HERE / "versions.json").read_text())


def command(args, cwd):
    subprocess.run(args, cwd=cwd, check=True)


def prepare(destination, runtime, dependencies):
    source = HERE / runtime
    producer = destination / runtime
    shutil.copytree(
        source,
        producer,
        ignore=shutil.ignore_patterns(
            "node_modules", ".venv", "target", "__pycache__", "producer"
        ),
    )
    if runtime == "node":
        manifest = producer / "package.json"
        data = json.loads(manifest.read_text())
        data["dependencies"] = dependencies
        manifest.write_text(json.dumps(data, indent=2) + "\n")
        command(["npm", "install", "--package-lock-only", "--ignore-scripts"], producer)
    elif runtime == "python":
        manifest = producer / "pyproject.toml"
        text = manifest.read_text()
        for name, version in dependencies.items():
            text, count = re.subn(
                rf'{re.escape(name)}==[^"\s]+', f"{name}=={version}", text
            )
            assert count == 1, name
        manifest.write_text(text)
        command(["uv", "lock"], producer)
    elif runtime == "rust":
        manifest = producer / "Cargo.toml"
        text = manifest.read_text()
        for name, version in dependencies.items():
            text, count = re.subn(
                rf'(?m)^({re.escape(name)}\s*=\s*(?:\{{\s*version\s*=\s*)?")=[^"]+',
                rf"\g<1>={version}",
                text,
            )
            assert count == 1, name
        manifest.write_text(text)
        command(["cargo", "update"], producer)
    else:
        for name, version in dependencies.items():
            command(["go", "mod", "edit", f"-require={name}@{version}"], producer)
        command(["go", "mod", "tidy"], producer)
    return producer


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=PROFILES, action="append")
    parser.add_argument(
        "--artifacts",
        type=Path,
        help="save resolved manifests, lockfiles, and successful results",
    )
    args = parser.parse_args()
    results = []
    for name in args.profile or PROFILES:
        profile = PROFILES[name]
        print(f"PROFILE {name}: {profile['dependencies']}", flush=True)
        with tempfile.TemporaryDirectory(prefix="lognorm-versions-") as temp:
            destination = Path(temp)
            producer = destination / profile["runtime"]
            result = {"profile": name, **profile, "result": "failed"}
            try:
                prepare(destination, profile["runtime"], profile["dependencies"])
                command(
                    [
                        sys.executable,
                        str(HERE / "run.py"),
                        "--runtime",
                        profile["runtime"],
                        "--producer-root",
                        str(destination),
                    ],
                    ROOT,
                )
                result["result"] = "passed"
                results.append(result)
            finally:
                if args.artifacts:
                    output = args.artifacts.resolve() / name
                    output.mkdir(parents=True, exist_ok=True)
                    for file in producer.iterdir():
                        if file.name in [
                            "package.json",
                            "package-lock.json",
                            "pyproject.toml",
                            "uv.lock",
                            "Cargo.toml",
                            "Cargo.lock",
                            "go.mod",
                            "go.sum",
                        ]:
                            shutil.copyfile(file, output / file.name)
                    (output / "result.json").write_text(
                        json.dumps(result, indent=2) + "\n"
                    )
    print(f"Verified {len(results)} version profiles", flush=True)


if __name__ == "__main__":
    main()
