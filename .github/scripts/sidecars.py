"""Download the helper programs (PHP, rclone) that ship with the app.

fetch [<target-triple>]   download, verify and unpack the programs for one target (default: this
                          computer) to apps/desktop/src-tauri/binaries/wm-<name>-<triple>[.exe]
lock                      download every listed archive, compute its SHA-256 and write it to
                          apps/desktop/sidecars.json (rclone is checked against its SHA256SUMS)

Versions, URLs and checksums live in apps/desktop/sidecars.json. Only the standard library is used,
so the script runs on all CI runners.
"""

import hashlib
import io
import json
import pathlib
import platform
import stat
import sys
import tarfile
import urllib.request
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
LOCK = ROOT / "apps/desktop/sidecars.json"
BINARIES = ROOT / "apps/desktop/src-tauri/binaries"


def fail(message: str) -> None:
    print(f"error: {message}", file=sys.stderr)
    sys.exit(1)


def host_triple() -> str:
    system = platform.system()
    machine = platform.machine().lower()
    arch = {"amd64": "x86_64", "x86_64": "x86_64", "arm64": "aarch64", "aarch64": "aarch64"}.get(machine)
    if arch is None:
        fail(f"unsupported machine {machine}")
    if system == "Windows":
        return f"{arch}-pc-windows-msvc"
    if system == "Darwin":
        return f"{arch}-apple-darwin"
    if system == "Linux":
        return f"{arch}-unknown-linux-gnu"
    fail(f"unsupported system {system}")
    return ""


def download(url: str) -> bytes:
    request = urllib.request.Request(url, headers={"User-Agent": "website-manager-ci"})
    with urllib.request.urlopen(request, timeout=120) as response:
        return response.read()


def extract(archive: bytes, url: str, member: str) -> bytes:
    if url.endswith(".zip"):
        with zipfile.ZipFile(io.BytesIO(archive)) as zip_file:
            return zip_file.read(member)
    if url.endswith(".tar.gz"):
        with tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz") as tar_file:
            extracted = tar_file.extractfile(member)
            if extracted is None:
                fail(f"{member} is not a file in {url}")
            return extracted.read()
    fail(f"unknown archive type: {url}")
    return b""


def fetch(triple: str) -> None:
    lock = json.loads(LOCK.read_text(encoding="utf-8"))
    BINARIES.mkdir(parents=True, exist_ok=True)
    for name, tool in lock["tools"].items():
        entry = tool["targets"].get(triple)
        if entry is None:
            fail(f"{name} has no build for {triple}")
        archive = download(entry["url"])
        actual = hashlib.sha256(archive).hexdigest()
        if actual != entry["sha256"]:
            fail(f"checksum mismatch for {entry['url']}: {actual}")
        suffix = ".exe" if "windows" in triple else ""
        target = BINARIES / f"wm-{name}-{triple}{suffix}"
        target.write_bytes(extract(archive, entry["url"], entry["member"]))
        target.chmod(target.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
        print(f"{name} {tool['version']} -> {target.relative_to(ROOT)}")


def lock() -> None:
    data = json.loads(LOCK.read_text(encoding="utf-8"))
    for name, tool in data["tools"].items():
        published = {}
        if tool.get("checksums"):
            for line in download(tool["checksums"]).decode().splitlines():
                parts = line.split()
                if len(parts) == 2:
                    published[parts[1]] = parts[0]
        for triple, entry in tool["targets"].items():
            archive = download(entry["url"])
            digest = hashlib.sha256(archive).hexdigest()
            file_name = entry["url"].rsplit("/", 1)[1]
            if published and published.get(file_name) != digest:
                fail(f"{file_name} does not match the published SHA256SUMS")
            extract(archive, entry["url"], entry["member"])
            entry["sha256"] = digest
            print(f"{name} {triple}: {digest}")
    LOCK.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    if len(sys.argv) >= 2 and sys.argv[1] == "fetch":
        fetch(sys.argv[2] if len(sys.argv) > 2 else host_triple())
    elif len(sys.argv) == 2 and sys.argv[1] == "lock":
        lock()
    else:
        fail(__doc__)
