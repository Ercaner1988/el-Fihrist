#!/usr/bin/env python3
"""Claude Desktop eklenti paketi (.mcpb) uretir.

Kullanim: python package-extension.py <binary-yolu> [cikti.mcpb]

claude-code-setup-rustified/package-extension.py'nin bire bir kopyasi,
yalnizca paket adi ve exe adi el-Fihrist'e gore degisti.
"""
import json
import pathlib
import sys
import zipfile

ROOT = pathlib.Path(__file__).parent
EXTRAS = ["icon.png", "README.md", "LICENSE"]
BIN_NAME = "ibnunnedim"

PLATFORM_BY_SUFFIX = {
    "windows": "win32",
    "macos": "darwin",
    "linux": "linux",
}


def platform_of(binary: pathlib.Path) -> str:
    name = binary.name.lower()
    for marker, platform in PLATFORM_BY_SUFFIX.items():
        if marker in name:
            return platform
    return "win32" if binary.suffix == ".exe" else sys.platform


def main() -> int:
    if len(sys.argv) < 2:
        print(__doc__)
        return 2

    binary = pathlib.Path(sys.argv[1])
    if not binary.is_file():
        print(f"HATA: binary bulunamadi: {binary}")
        return 1

    manifest = json.loads((ROOT / "manifest.json").read_text(encoding="utf-8"))
    platform = platform_of(binary)
    entry = f"{BIN_NAME}.exe" if platform == "win32" else BIN_NAME

    manifest["server"]["entry_point"] = entry
    manifest["server"]["mcp_config"]["command"] = "${__dirname}/" + entry
    manifest.setdefault("compatibility", {})["platforms"] = [platform]

    friendly = {"win32": "windows", "darwin": "macos", "linux": "linux"}
    default_name = f"{manifest['name']}-{friendly.get(platform, platform)}.mcpb"
    out = pathlib.Path(sys.argv[2]) if len(sys.argv) > 2 else ROOT / default_name

    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        z.writestr("manifest.json", json.dumps(manifest, indent=2, ensure_ascii=False))
        for name in EXTRAS:
            path = ROOT / name
            if path.is_file():
                z.write(path, name)
            else:
                print(f"UYARI: atlandi (yok): {name}")
        info = zipfile.ZipInfo(entry)
        info.compress_type = zipfile.ZIP_DEFLATED
        info.create_system = 3  # Unix
        info.external_attr = 0o755 << 16
        z.writestr(info, binary.read_bytes())

    size = out.stat().st_size / 1_048_576
    print(f"OK: {out}  ({size:.1f} MB, platform={platform}, entry_point={entry})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
