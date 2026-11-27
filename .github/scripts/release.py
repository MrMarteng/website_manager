"""Release helpers for the GitHub workflow.

check <tag>                 versions in Cargo.toml, tauri.conf.json and package.json match the tag,
                            and CHANGELOG.md has a section for it
notes <tag> <out>           write the release notes (changelog section plus notice) to <out>
collect <tag> <in> <out>    copy the build artifacts from <in> to <out> with uniform names and
                            write SHA256SUMS
"""

import hashlib
import json
import pathlib
import re
import shutil
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]

NOTICE = """> **Pre-release, unsigned installers.** Windows SmartScreen shows "Windows protected your PC" →
> **More info → Run anyway**; on macOS open the app with right-click → **Open** or allow it under
> System Settings → Privacy & Security. PHP and rclone are not bundled yet, see
> [docs/windows-test.md](https://github.com/MrMarteng/website_manager/blob/main/docs/windows-test.md).
> Verify downloads with `SHA256SUMS`.
"""

# (artifact folder prefix, glob inside the artifact, file name pattern, executable)
FILES = [
    ("website-manager-Windows-", "bundle/msi/*.msi", "website-manager_{v}_windows_x64.msi", False),
    ("website-manager-Windows-", "bundle/nsis/*-setup.exe", "website-manager_{v}_windows_x64-setup.exe", False),
    ("website-manager-Windows-", "website-manager.exe", "website-manager-cli_{v}_windows_x64.exe", False),
    ("website-manager-macOS-", "bundle/dmg/*.dmg", "website-manager_{v}_macos_arm64.dmg", False),
    ("website-manager-macOS-", "website-manager", "website-manager-cli_{v}_macos_arm64", True),
    ("website-manager-Linux-", "bundle/appimage/*.AppImage", "website-manager_{v}_linux_x64.AppImage", True),
    ("website-manager-Linux-", "bundle/deb/*.deb", "website-manager_{v}_linux_amd64.deb", False),
    ("website-manager-Linux-", "website-manager", "website-manager-cli_{v}_linux_x64", True),
]


def fail(message: str) -> None:
    print(f"error: {message}", file=sys.stderr)
    sys.exit(1)


def version_from_tag(tag: str) -> str:
    if not re.fullmatch(r"v\d+\.\d+\.\d+", tag):
        fail(f"tag {tag!r} is not of the form vX.Y.Z")
    return tag[1:]


def changelog_section(version: str) -> str:
    text = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
    match = re.search(
        rf"^## \[{re.escape(version)}\][^\n]*\n(.*?)(?=^## \[|^\[[^\]]+\]: )",
        text,
        flags=re.S | re.M,
    )
    if not match or not match.group(1).strip():
        fail(f"CHANGELOG.md has no section for {version}")
    return match.group(1).strip()


def check(tag: str) -> None:
    version = version_from_tag(tag)
    cargo = re.search(
        r'^\[workspace\.package\]\s*\nversion\s*=\s*"([^"]+)"',
        (ROOT / "Cargo.toml").read_text(encoding="utf-8"),
        flags=re.M,
    )
    found = {
        "Cargo.toml": cargo.group(1) if cargo else None,
        "tauri.conf.json": json.loads(
            (ROOT / "apps/desktop/src-tauri/tauri.conf.json").read_text(encoding="utf-8")
        )["version"],
        "package.json": json.loads(
            (ROOT / "apps/desktop/package.json").read_text(encoding="utf-8")
        )["version"],
    }
    wrong = {name: value for name, value in found.items() if value != version}
    if wrong:
        fail(f"tag {tag} does not match the version in {wrong}")
    changelog_section(version)
    print(f"version {version} is consistent and has a changelog section")


def notes(tag: str, out: str) -> None:
    version = version_from_tag(tag)
    pathlib.Path(out).write_text(f"{NOTICE}\n{changelog_section(version)}\n", encoding="utf-8")


def collect(tag: str, source: str, target: str) -> None:
    version = version_from_tag(tag)
    source_dir = pathlib.Path(source)
    target_dir = pathlib.Path(target)
    target_dir.mkdir(parents=True, exist_ok=True)
    for prefix, pattern, name, executable in FILES:
        folders = sorted(source_dir.glob(f"{prefix}*"))
        if len(folders) != 1:
            fail(f"expected one artifact {prefix}*, found {len(folders)}")
        matches = sorted(folders[0].glob(pattern))
        if len(matches) != 1:
            fail(f"expected one file {pattern} in {folders[0].name}, found {len(matches)}")
        destination = target_dir / name.format(v=version)
        shutil.copyfile(matches[0], destination)
        if executable:
            destination.chmod(0o755)
        print(f"{matches[0].relative_to(source_dir)} -> {destination.name}")
    sums = []
    for file in sorted(target_dir.iterdir()):
        if file.name != "SHA256SUMS":
            sums.append(f"{hashlib.sha256(file.read_bytes()).hexdigest()}  {file.name}")
    (target_dir / "SHA256SUMS").write_text("\n".join(sums) + "\n", encoding="utf-8")


if __name__ == "__main__":
    commands = {"check": (check, 1), "notes": (notes, 2), "collect": (collect, 3)}
    if len(sys.argv) < 2 or sys.argv[1] not in commands:
        fail(__doc__)
    function, count = commands[sys.argv[1]]
    if len(sys.argv) != count + 2:
        fail(__doc__)
    function(*sys.argv[2:])
