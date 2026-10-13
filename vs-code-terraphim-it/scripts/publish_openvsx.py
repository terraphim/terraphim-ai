#!/usr/bin/env python3
"""Publish qualified bytes using explicit Open VSX first-version or OIDC mode."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
from urllib.error import HTTPError
from urllib.request import Request, urlopen

from verify_registry import identity, qualify


def require_first_publication(status, metadata, manifest):
    """Only the registry's exact missing-extension response permits bootstrap."""
    publisher, name, _ = identity(manifest)
    expected = f"Extension not found: {publisher}.{name}"
    if status != 404 or not isinstance(metadata, dict) or metadata.get("error") != expected:
        raise ValueError("Open VSX bootstrap requires an absent extension; use trusted publishing for existing versions")


def publication_environment(bootstrap, environment):
    """Keep the scoped credential in the child environment, never CLI arguments."""
    child = dict(environment)
    token = child.pop("OVSX_BOOTSTRAP_PAT", "")
    # ovsx gives PATs precedence over OIDC; neither inherited credentials nor a
    # failed OIDC exchange may silently select a different publication method.
    child.pop("OVSX_PAT", None)
    if bootstrap:
        if not token.strip():
            raise ValueError("Explicit bootstrap requires the environment secret OVSX_BOOTSTRAP_PAT")
        child["OVSX_PAT"] = token
    return child


def publication_command(package, bootstrap):
    """Construct a shell-free command using only the qualified package path."""
    command = ["npm", "exec", "--", "ovsx", "publish", str(package.resolve()), "--pre-release"]
    if not bootstrap:
        command.append("--trusted-publishing")
    return command


def registry_metadata(manifest):
    publisher, name, _ = identity(manifest)
    request = Request(f"https://open-vsx.org/api/{publisher}/{name}",
                      headers={"Accept": "application/json", "User-Agent": "Terraphim-release-verifier/1.0"})
    try:
        with urlopen(request, timeout=60) as response:
            status, body = response.status, response.read()
    except HTTPError as error:
        status, body = error.code, error.read()
    try:
        return status, json.loads(body)
    except ValueError as error:
        raise ValueError("Open VSX returned invalid metadata; publication refused") from error


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--vsix", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--bootstrap", action="store_true",
                        help="Explicit first-version PAT bootstrap; never an automatic OIDC fallback")
    args = parser.parse_args()
    try:
        manifest, _ = qualify(args.vsix, args.manifest)
        command = publication_command(args.vsix, args.bootstrap)
        environment = publication_environment(args.bootstrap, os.environ)
        if args.bootstrap:
            require_first_publication(*registry_metadata(manifest), manifest)
        # ubs:ignore -- False positive: credential flows to env only; fixed argv and shell=False.
        subprocess.run(command, env=environment, check=True, shell=False, timeout=180)
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        print(f"publish_openvsx: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
