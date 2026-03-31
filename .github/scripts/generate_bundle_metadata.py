#!/usr/bin/env python3
"""Generate canonical bundle metadata for published binary scopes.

This script is the single supported generator for:
- provenance-policy.json
- sbom.spdx.json
- checksums.sha256

It writes UTF-8 without BOM, uses LF line endings, and emits deterministic
file ordering.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


CHECKSUM_MANIFEST = "checksums.sha256"
SBOM_FILE = "sbom.spdx.json"
PROVENANCE_POLICY_FILE = "provenance-policy.json"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest().lower()


def write_text(path: Path, content: str) -> None:
    with path.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(content)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Generate canonical bundle metadata.")
    parser.add_argument("--bundle-dir", required=True)
    parser.add_argument("--bundle-name", required=True)
    parser.add_argument("--namespace", required=True)
    parser.add_argument("--repo", required=True)
    parser.add_argument("--signer-workflow", required=True)
    parser.add_argument("--source-ref", required=True)
    parser.add_argument(
        "--predicate-type",
        default="https://slsa.dev/provenance/v1",
    )
    return parser.parse_args()


def generate_provenance_policy(bundle_dir: Path, args: argparse.Namespace) -> None:
    policy = {
        "schema_version": 1,
        "required": True,
        "repo": args.repo,
        "signer_workflow": args.signer_workflow,
        "source_ref": args.source_ref,
        "predicate_type": args.predicate_type,
    }
    write_text(
        bundle_dir / PROVENANCE_POLICY_FILE,
        json.dumps(policy, indent=2) + "\n",
    )


def bundled_files(bundle_dir: Path, *, include_sbom: bool) -> list[Path]:
    excluded = {CHECKSUM_MANIFEST}
    if not include_sbom:
        excluded.add(SBOM_FILE)
    return sorted(
        (
            path
            for path in bundle_dir.iterdir()
            if path.is_file() and path.name not in excluded
        ),
        key=lambda path: path.name,
    )


def generate_sbom(bundle_dir: Path, args: argparse.Namespace) -> None:
    files = []
    for path in bundled_files(bundle_dir, include_sbom=False):
        files.append(
            {
                "SPDXID": f"SPDXRef-File-{path.name.replace('.', '-').replace('/', '-')}",
                "fileName": f"./{path.name}",
                "checksums": [
                    {
                        "algorithm": "SHA256",
                        "checksumValue": sha256(path),
                    }
                ],
            }
        )

    document = {
        "spdxVersion": "SPDX-2.3",
        "dataLicense": "CC0-1.0",
        "SPDXID": "SPDXRef-DOCUMENT",
        "name": args.bundle_name,
        "documentNamespace": args.namespace,
        "creationInfo": {
            "created": "2026-03-29T00:00:00Z",
            "creators": ["Tool: prd-to-product-agents bundle metadata"],
        },
        "files": files,
    }
    write_text(bundle_dir / SBOM_FILE, json.dumps(document, indent=2) + "\n")


def generate_checksums(bundle_dir: Path) -> None:
    lines = []
    for path in bundled_files(bundle_dir, include_sbom=True):
        lines.append(f"{sha256(path)}  {path.name}")
    write_text(bundle_dir / CHECKSUM_MANIFEST, "\n".join(lines) + "\n")


def main() -> int:
    args = parse_args()
    bundle_dir = Path(args.bundle_dir).resolve()
    if not bundle_dir.is_dir():
        raise SystemExit(f"bundle directory does not exist: {bundle_dir}")

    generate_provenance_policy(bundle_dir, args)
    generate_sbom(bundle_dir, args)
    generate_checksums(bundle_dir)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
