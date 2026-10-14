#!/usr/bin/env python3
"""Read the federated publisher's identity; refuse an unconfirmed profile.

Only fixed Azure CLI metadata queries are executed. Tokens, full profiles and
CLI error text are never written to stdout, stderr or the retained receipt.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import uuid

from verify_vsix import check_tag

ROOT = Path(__file__).resolve().parents[1]
RESOURCE = "499b84ac-1321-427f-aa17-267ca6975798"
PROFILE_URL = "https://app.vssps.visualstudio.com/_apis/profile/profiles/me?api-version=7.1"
ACCOUNT_COMMAND = ["az", "account", "show", "--query",
                   "{tenantId:tenantId,userName:user.name,userType:user.type}",
                   "--output", "json", "--only-show-errors"]
PROFILE_COMMAND = ["az", "rest", "--method", "get", "--url", PROFILE_URL,
                   "--resource", RESOURCE, "--query", "{id:id}",
                   "--output", "json", "--only-show-errors"]


class Refusal(ValueError):
    """A fixed public error code, never an external response or command output."""


def identifier(value):
    if not isinstance(value, str) or re.fullmatch(
            r"[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}", value) is None:
        raise Refusal("INVALID_IDENTITY_IDENTIFIER")
    result = uuid.UUID(value)
    if result.int == 0:
        raise Refusal("INVALID_IDENTITY_IDENTIFIER")
    return str(result)


def configuration(environment, version):
    """Admit only this repository's exact release tag and explicit identities."""
    if (environment.get("GITHUB_REPOSITORY") != "terraphim/terraphim-ai"
            or environment.get("GITHUB_REF_TYPE") != "tag"
            or re.fullmatch(r"[0-9a-f]{40}", environment.get("GITHUB_SHA", "")) is None):
        raise Refusal("INVALID_RELEASE_CONTEXT")
    try:
        check_tag(environment.get("GITHUB_REF_NAME", ""), version)
    except ValueError:
        raise Refusal("INVALID_RELEASE_CONTEXT") from None
    client = identifier(environment.get("VSCE_AZURE_CLIENT_ID"))
    tenant = identifier(environment.get("VSCE_AZURE_TENANT_ID"))
    expected = environment.get("VSCE_MARKETPLACE_PROFILE_ID", "")
    return client, tenant, identifier(expected) if expected else None


def parse_metadata(raw):
    """Reject ambiguous or oversized metadata without echoing any input."""
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise Refusal("INVALID_METADATA")
            result[key] = value
        return result

    if not isinstance(raw, bytes) or len(raw) > 65536:
        raise Refusal("INVALID_METADATA")
    try:
        value = json.loads(raw, object_pairs_hook=pairs)
    except (ValueError, UnicodeError):
        raise Refusal("INVALID_METADATA") from None
    if not isinstance(value, dict):
        raise Refusal("INVALID_METADATA")
    return value


def run_metadata(command, timeout=60):
    """Capture a real child process privately and expose only validated JSON."""
    try:
        result = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                timeout=timeout, check=False)
    except subprocess.TimeoutExpired:
        raise Refusal("IDENTITY_QUERY_TIMEOUT") from None
    except OSError:
        raise Refusal("IDENTITY_QUERY_UNAVAILABLE") from None
    if result.returncode:
        raise Refusal("IDENTITY_QUERY_FAILED")
    return parse_metadata(result.stdout)


def verify_account(account, client, tenant):
    if (not isinstance(account, dict) or set(account) != {"tenantId", "userName", "userType"}
            or account["userType"] != "servicePrincipal"):
        raise Refusal("UNEXPECTED_ACCOUNT_TYPE")
    if identifier(account["tenantId"]) != tenant or identifier(account["userName"]) != client:
        raise Refusal("AUTHENTICATED_IDENTITY_MISMATCH")


def profile_id(profile):
    if not isinstance(profile, dict) or set(profile) != {"id"}:
        raise Refusal("INVALID_PROFILE_METADATA")
    return identifier(profile["id"])


def confirm_profile(actual, expected):
    if expected is None:
        raise Refusal("PUBLISHER_MEMBERSHIP_CONFIRMATION_REQUIRED")
    if actual != expected:
        raise Refusal("PUBLISHER_PROFILE_MISMATCH")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", type=Path, required=True)
    args = parser.parse_args()
    result = {"schema": 1, "status": "NOT_READY",
              "scope": "Identity admission only; publisher membership and publication require separate verification"}
    try:
        # Retain earlier evidence, including symlinks to absent destinations.
        with args.receipt.open("x", encoding="utf-8") as output:
            try:
                version = json.loads((ROOT / "package.json").read_text())["version"]
                client, tenant, expected = configuration(os.environ, version)
                verify_account(run_metadata(ACCOUNT_COMMAND), client, tenant)
                actual = profile_id(run_metadata(PROFILE_COMMAND))
                result.update(clientId=client, tenantId=tenant, profileId=actual,
                              sourceCommit=os.environ["GITHUB_SHA"], tag=os.environ["GITHUB_REF_NAME"])
                confirm_profile(actual, expected)
                result["status"] = "IDENTITY_VERIFIED_READY"
            except Refusal as error:
                result["errorCode"] = str(error)
            except (OSError, ValueError, KeyError, TypeError):
                result["errorCode"] = "INVALID_LOCAL_CONFIGURATION"
            output.write(json.dumps(result, indent=2) + "\n")
    except OSError:
        print("marketplace_identity: RECEIPT_UNAVAILABLE", file=sys.stderr)
        return 1
    if result["status"] != "IDENTITY_VERIFIED_READY":
        print("marketplace_identity: " + result["errorCode"], file=sys.stderr)
        return 1
    print("marketplace_identity: IDENTITY_VERIFIED_READY (publication not yet verified)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
