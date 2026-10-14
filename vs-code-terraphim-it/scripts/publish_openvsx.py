"""Publish qualified bytes only through Open VSX trusted publishing."""
import argparse
import os
import subprocess
import sys
from pathlib import Path

from verify_registry import qualify


def publication_environment(environment):
    """Remove PAT overrides: a failed OIDC exchange must fail closed."""
    child = dict(environment)
    child.pop("OVSX_PAT", None)
    child.pop("OVSX_BOOTSTRAP_PAT", None)
    return child


def publication_command(package):
    """Construct a shell-free OIDC command using only qualified package bytes."""
    return ["npm", "exec", "--", "ovsx", "publish", str(package.resolve()),
            "--pre-release", "--trusted-publishing"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--vsix", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    args = parser.parse_args()
    try:
        qualify(args.vsix, args.manifest)
        command = publication_command(args.vsix)
        environment = publication_environment(os.environ)
        # ubs:ignore -- False positive: credential flows to env only; fixed argv and shell=False.
        subprocess.run(command, env=environment, check=True, shell=False, timeout=180)
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        print(f"publish_openvsx: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
