#!/usr/bin/env python3
"""Copy signed updater artifacts and create the GitHub Release latest.json."""
import argparse
import datetime
import json
import pathlib
import platform
import shutil
import urllib.parse


def prepare(root, out, target, notes="优化体验与修复问题。", base=None):
    config = json.loads((root / "src-tauri/tauri.conf.json").read_text())
    version = config["version"]
    bundles = root / "src-tauri/target/release/bundle"
    patterns = {"darwin": "macos/*.app.tar.gz", "windows": "nsis/*.exe", "linux": "appimage/*.AppImage"}
    os_name = target.split("-")[0]
    candidates = [p for p in bundles.glob(patterns[os_name]) if pathlib.Path(str(p) + ".sig").exists()]
    if len(candidates) != 1:
        raise ValueError("Expected exactly one signed updater artifact for " + target)
    artifact = candidates[0]
    signature = pathlib.Path(str(artifact) + ".sig").read_text().strip()
    if not signature:
        raise ValueError("Missing updater signature")
    manifest = json.loads(base.read_text()) if base else {"version": version, "platforms": {}}
    if manifest["version"] != version:
        raise ValueError("Cannot merge update manifests for different versions")
    out.mkdir(parents=True, exist_ok=True)
    # Platform suffixes prevent Intel/Apple Silicon artifacts overwriting each other.
    suffix = {"darwin": ".app.tar.gz", "windows": "-setup.exe", "linux": ".AppImage"}[os_name]
    name = "Mickey-v" + version + "-" + target + suffix
    shutil.copy2(artifact, out / name)
    shutil.copy2(str(artifact) + ".sig", out / (name + ".sig"))
    manifest.update(notes=notes, pub_date=datetime.datetime.now(datetime.timezone.utc).isoformat())
    manifest["platforms"][target] = {
        "signature": signature,
        "url": "https://github.com/Zimzheng/meowMickey/releases/download/v" + version + "/" + urllib.parse.quote(name),
    }
    (out / "latest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
    return manifest


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=pathlib.Path, required=True)
    parser.add_argument("--target", default={"Darwin": "darwin", "Windows": "windows", "Linux": "linux"}[platform.system()] + "-" + {"arm64": "aarch64", "AMD64": "x86_64"}.get(platform.machine(), platform.machine()))
    parser.add_argument("--notes", default="优化体验与修复问题。")
    parser.add_argument("--merge", type=pathlib.Path, help="Add this platform to another platform's latest.json")
    args = parser.parse_args()
    prepare(pathlib.Path(__file__).resolve().parent.parent, args.out, args.target, args.notes, args.merge)
    print("Signed update artifacts and latest.json prepared in", args.out)
