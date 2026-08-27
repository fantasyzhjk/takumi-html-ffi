#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import os
import platform
import shutil
import subprocess
from pathlib import Path

UNIFFI_BINDGEN_VERSION = "0.29.5"


def run(command: list[str], *, cwd: Path | None = None) -> None:
    print(f"[DEBUG] Running: {' '.join(str(c) for c in command)}", flush=True)
    subprocess.run(command, cwd=cwd, check=True)


def clean_dir(path: Path) -> None:
    if path.exists():
        shutil.rmtree(path)
    path.mkdir(parents=True, exist_ok=True)


def locate_command(*candidates: str | Path) -> str:
    for candidate in candidates:
        if isinstance(candidate, Path):
            if candidate.exists():
                return str(candidate)
            continue

        found = shutil.which(candidate)
        if found:
            return found

    joined = ", ".join(str(candidate) for candidate in candidates)
    raise RuntimeError(f"Unable to locate required command. Tried: {joined}")


def tool_binary_path(tool_root: Path, binary_name: str) -> Path:
    suffix = ".exe" if os.name == "nt" else ""
    return tool_root / "bin" / f"{binary_name}{suffix}"


def ensure_local_uniffi_bindgen(crate_dir: Path, version: str = UNIFFI_BINDGEN_VERSION) -> str:
    tool_root = crate_dir / ".tools" / f"uniffi-bindgen-{version}"
    binary_path = tool_binary_path(tool_root, "uniffi-bindgen")
    if not binary_path.exists():
        tool_project = Path(__file__).resolve().parent / "uniffi-bindgen"
        run(
            [
                "cargo",
                "install",
                "--locked",
                "--root",
                str(tool_root),
                "--path",
                str(tool_project),
                "--bin",
                "uniffi-bindgen",
            ]
        )
    return str(binary_path)


def command_version(command: str) -> str | None:
    try:
        result = subprocess.run(
            [command, "--version"],
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError:
        return None
    if result.returncode != 0:
        return None
    output = (result.stdout or result.stderr).strip()
    return output.split()[-1] if output else None


def locate_uniffi_bindgen(crate_dir: Path) -> str:
    candidates: tuple[str | Path, ...] = (
        tool_binary_path(
            crate_dir / ".tools" / f"uniffi-bindgen-{UNIFFI_BINDGEN_VERSION}",
            "uniffi-bindgen",
        ),
        tool_binary_path(Path.home() / ".cargo", "uniffi-bindgen"),
        "uniffi-bindgen",
    )

    for candidate in candidates:
        try:
            command = locate_command(candidate)
        except RuntimeError:
            continue
        if command_version(command) == UNIFFI_BINDGEN_VERSION:
            return command
        print(
            f"[DEBUG] Ignoring UniFFI bindgen with unexpected version: {command}",
            flush=True,
        )

    command = ensure_local_uniffi_bindgen(crate_dir)
    actual_version = command_version(command)
    if actual_version != UNIFFI_BINDGEN_VERSION:
        raise RuntimeError(
            f"Expected uniffi-bindgen {UNIFFI_BINDGEN_VERSION}, found {actual_version or 'unknown'}"
        )
    return command


def detect_host_rid() -> str:
    system = platform.system().lower()
    machine = platform.machine().lower()

    arch_map = {
        "x86_64": "x64",
        "amd64": "x64",
        "aarch64": "arm64",
        "arm64": "arm64",
    }
    architecture = arch_map.get(machine)
    if architecture is None:
        raise RuntimeError(f"Unsupported host architecture: {machine}")

    if system == "linux":
        os_name = "linux"
    elif system == "darwin":
        os_name = "osx"
    elif system == "windows":
        os_name = "win"
    else:
        raise RuntimeError(f"Unsupported host OS: {system}")

    return f"{os_name}-{architecture}"


def possible_native_filenames(base_name: str) -> tuple[str, str, str]:
    return (
        f"lib{base_name}.so",
        f"lib{base_name}.dylib",
        f"{base_name}.dll",
    )


def infer_rid_from_target_name(target_name: str) -> str | None:
    lowered = target_name.lower()
    if lowered in {"debug", "release", ".fingerprint", "build", "deps", "examples", "incremental"}:
        return None

    if lowered.startswith("x86_64"):
        architecture = "x64"
    elif lowered.startswith("aarch64"):
        architecture = "arm64"
    else:
        return None

    if "windows" in lowered:
        os_name = "win"
    elif "linux" in lowered:
        os_name = "linux"
    elif "apple-darwin" in lowered:
        os_name = "osx"
    else:
        return None

    return f"{os_name}-{architecture}"


def find_host_native_library(target_dir: Path, base_name: str) -> Path:
    release_dir = target_dir / "release"
    for file_name in possible_native_filenames(base_name):
        candidate = release_dir / file_name
        if candidate.exists():
            return candidate

    raise FileNotFoundError(
        f"Unable to find a host native library for {base_name!r} under {release_dir}"
    )


def collect_native_artifacts(target_dir: Path, base_name: str) -> dict[str, Path]:
    artifacts: dict[str, Path] = {}
    if not target_dir.exists():
        return artifacts

    host_rid = detect_host_rid()
    file_names = possible_native_filenames(base_name)

    for file_name in file_names:
        candidate = target_dir / "release" / file_name
        if candidate.exists():
            artifacts[host_rid] = candidate

    for child in target_dir.iterdir():
        if not child.is_dir():
            continue
        rid = infer_rid_from_target_name(child.name)
        if rid is None:
            continue
        for file_name in file_names:
            candidate = child / "release" / file_name
            if candidate.exists():
                artifacts[rid] = candidate

    return artifacts


def stage_native_artifacts(target_dir: Path, base_name: str, destination_root: Path) -> None:
    clean_dir(destination_root)
    artifacts = collect_native_artifacts(target_dir, base_name)
    if not artifacts:
        raise FileNotFoundError(f"No native artifacts found for {base_name!r} under {target_dir}")

    for rid, source_path in artifacts.items():
        out_dir = destination_root / rid
        out_dir.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source_path, out_dir / source_path.name)


def copy_matching_files(source_root: Path, destination_root: Path, suffix: str) -> None:
    destination_root.mkdir(parents=True, exist_ok=True)
    matched = False
    copied_paths: set[Path] = set()

    for source_path in source_root.rglob(f"*{suffix}"):
        relative_path = source_path.relative_to(source_root)
        target_path = destination_root / relative_path
        target_path.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source_path, target_path)
        copied_paths.add(target_path.resolve())
        matched = True

    if not matched:
        raise FileNotFoundError(f"No files ending with {suffix!r} found under {source_root}")

    for existing_path in destination_root.rglob(f"*{suffix}"):
        if existing_path.resolve() in copied_paths:
            continue
        existing_path.unlink()


def build_release(manifest_path: Path) -> None:
    run(["cargo", "build", "--manifest-path", str(manifest_path), "--release"])


def cargo_target_dir(manifest_path: Path) -> Path:
    command = [
        "cargo",
        "metadata",
        "--manifest-path",
        str(manifest_path),
        "--format-version",
        "1",
        "--no-deps",
    ]
    print(f"[DEBUG] Running: {' '.join(command)}", flush=True)
    metadata = json.loads(subprocess.check_output(command, text=True))
    return Path(metadata["target_directory"])


def find_crate_dir(project_dir: Path, explicit_crate_dir: Path | None) -> Path:
    if explicit_crate_dir:
        return explicit_crate_dir.resolve()
    for parent in [project_dir] + list(project_dir.parents):
        if (parent / "uniffi.toml").exists() and (parent / "Cargo.toml").exists():
            return parent
    raise RuntimeError(f"Could not auto-detect crate directory for project at {project_dir}")


def get_cdylib_name(crate_dir: Path) -> str:
    # Read uniffi.toml to get cdylib_name, fallback to crate directory name with underscores
    uniffi_toml = crate_dir / "uniffi.toml"
    if uniffi_toml.exists():
        for line in uniffi_toml.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if line.startswith("cdylib_name"):
                parts = line.split("=", 1)
                if len(parts) == 2:
                    return parts[1].strip().strip('"').strip("'")
    return crate_dir.name.replace("-", "_")


def prepare_csharp(crate_dir: Path, project_dir: Path) -> None:
    manifest_path = crate_dir / "Cargo.toml"
    config_path = crate_dir / "uniffi.toml"
    lib_name = get_cdylib_name(crate_dir)
    generated_bindings_dir = project_dir / "Generated" / "Bindings"
    generated_native_dir = project_dir / "Generated" / "Native"
    scratch_dir = project_dir / "obj" / "uniffi-bindgen" / "csharp"

    build_release(manifest_path)
    target_dir = cargo_target_dir(manifest_path)
    native_library = find_host_native_library(target_dir, lib_name)

    clean_dir(scratch_dir)
    bindgen_cs = locate_command(Path.home() / ".cargo/bin/uniffi-bindgen-cs", "uniffi-bindgen-cs")
    run(
        [
            bindgen_cs,
            "--no-format",
            "--library",
            str(native_library),
            "--config",
            str(config_path),
            "--out-dir",
            str(scratch_dir),
        ]
    )

    generated_component = scratch_dir / f"{lib_name}.cs"
    if not generated_component.exists():
        raise FileNotFoundError(
            f"Expected generated C# component file at {generated_component}, but it was not created"
        )

    pascal_name = "".join(part.capitalize() for part in lib_name.split("_"))
    clean_dir(generated_bindings_dir)
    shutil.copy2(generated_component, generated_bindings_dir / f"{pascal_name}.Generated.cs")
    stage_native_artifacts(target_dir, lib_name, generated_native_dir)


def prepare_kotlin(crate_dir: Path, project_dir: Path) -> None:
    manifest_path = crate_dir / "Cargo.toml"
    config_path = crate_dir / "uniffi.toml"
    lib_name = get_cdylib_name(crate_dir)
    generated_sources_dir = project_dir / "Generated" / "Kotlin"
    generated_resources_dir = project_dir / "Generated" / "Resources" / "native"
    scratch_dir = project_dir / "build" / "uniffi-bindgen" / "kotlin"

    build_release(manifest_path)
    target_dir = cargo_target_dir(manifest_path)
    native_library = find_host_native_library(target_dir, lib_name)

    clean_dir(scratch_dir)
    uniffi_bindgen = locate_uniffi_bindgen(crate_dir)
    run(
        [
            uniffi_bindgen,
            "generate",
            "--no-format",
            "--library",
            str(native_library),
            "--language",
            "kotlin",
            "--config",
            str(config_path),
            "--out-dir",
            str(scratch_dir),
        ]
    )

    copy_matching_files(scratch_dir, generated_sources_dir, ".kt")
    stage_native_artifacts(target_dir, lib_name, generated_resources_dir)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Prepare packaged UniFFI bindings and native resources.")
    parser.add_argument("--language", required=True, choices=("csharp", "kotlin"))
    parser.add_argument("--project-dir", required=True, help="Target project directory (C# or Kotlin)")
    parser.add_argument("--crate-dir", default=None, help="Optional crate directory (defaults to auto-detected parent)")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    project_dir = Path(args.project_dir).resolve()
    project_dir.mkdir(parents=True, exist_ok=True)
    crate_dir = find_crate_dir(project_dir, Path(args.crate_dir) if args.crate_dir else None)

    if args.language == "csharp":
        prepare_csharp(crate_dir, project_dir)
    elif args.language == "kotlin":
        prepare_kotlin(crate_dir, project_dir)
    else:
        raise RuntimeError(f"Unsupported language: {args.language}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
