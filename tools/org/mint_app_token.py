#!/usr/bin/env python3
"""Mint a GitHub App installation access token from the app private key.

Dependency-free (stdlib + openssl, available on macOS, Linux, and CI). Prints the
token on stdout so it can be piped to `gh auth login --with-token` or exported as
GH_TOKEN. The PEM private key never leaves the caller's machine.

Usage:
  python3 tools/org/mint_app_token.py \\
    --app-id 1234567 \\
    --key /secure/path/ramen-implementer.private-key.pem \\
    --repo maxwellsantoro/RamenOS \\
    | gh auth login --with-token

  # just resolve and print the installation id, then exit:
  python3 tools/org/mint_app_token.py --app-id ... --key ... --print-installation-id
"""

from __future__ import annotations

import argparse
import base64
import json
import subprocess
import sys
import time
import urllib.error
import urllib.request

GH_API = "https://api.github.com"


def b64url(data: bytes) -> str:
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode("ascii")


def make_jwt(app_id: str, key_path: str) -> str:
    header = b64url(b'{"alg":"RS256","typ":"JWT"}')
    now = int(time.time())
    payload = b64url(
        json.dumps({"iat": now - 60, "exp": now + 9 * 60, "iss": app_id}, separators=(",", ":")).encode()
    )
    signing_input = f"{header}.{payload}".encode()
    signature = subprocess.run(
        ["openssl", "dgst", "-sha256", "-sign", key_path],
        input=signing_input,
        capture_output=True,
        check=True,
    ).stdout
    return f"{header}.{payload}.{b64url(signature)}"


def api(method: str, path: str, jwt: str | None = None) -> tuple[int, dict]:
    url = f"{GH_API}{path}"
    headers = {
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
    }
    if jwt:
        headers["Authorization"] = f"Bearer {jwt}"
    req = urllib.request.Request(url, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req) as resp:
            body = resp.read().decode()
            return resp.status, json.loads(body) if body else {}
    except urllib.error.HTTPError as exc:
        body = exc.read().decode() or "{}"
        return exc.code, json.loads(body) if body else {}


def main() -> int:
    parser = argparse.ArgumentParser(description="Mint a GitHub App installation access token.")
    parser.add_argument("--app-id", required=True)
    parser.add_argument("--key", required=True, help="path to the app PEM private key")
    parser.add_argument("--repo", default="maxwellsantoro/RamenOS", help="repo to resolve the installation for")
    parser.add_argument("--installation-id", help="skip discovery if you already know the installation id")
    parser.add_argument("--print-installation-id", action="store_true", help="resolve and print the installation id, then exit")
    args = parser.parse_args()

    jwt = make_jwt(args.app_id, args.key)
    status, me = api("GET", "/app", jwt=jwt)
    if status != 200:
        print(f"app authentication failed (HTTP {status}): {me}", file=sys.stderr)
        print("check --app-id and that --key is the correct PEM", file=sys.stderr)
        return 1

    if args.installation_id:
        installation_id = args.installation_id
    else:
        status, obj = api("GET", f"/repos/{args.repo}/installation", jwt=jwt)
        if status != 200:
            print(f"could not resolve installation for {args.repo} (HTTP {status}): {obj}", file=sys.stderr)
            print("ensure the app is installed on this repo, or pass --installation-id", file=sys.stderr)
            return 1
        installation_id = str(obj["id"])

    if args.print_installation_id:
        print(installation_id)
        return 0

    # Mint an installation access token (valid ~1 hour).
    url = f"{GH_API}/app/installations/{installation_id}/access_tokens"
    req = urllib.request.Request(
        url,
        data=b"{}",
        headers={
            "Authorization": f"Bearer {jwt}",
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
            "Content-Type": "application/json",
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(req) as resp:
            token = json.loads(resp.read().decode())["token"]
    except urllib.error.HTTPError as exc:
        print(f"token mint failed (HTTP {exc.code}): {exc.read().decode()}", file=sys.stderr)
        return 1

    print(token)
    return 0


if __name__ == "__main__":
    sys.exit(main())
