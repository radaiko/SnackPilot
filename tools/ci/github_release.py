#!/usr/bin/env python3
"""Publish a built desktop release as a GitHub Release.

GitHub Actions does the building; this script creates the release for the tag
(GitHub creates the tag at the given commit if it does not exist yet), generates
the notes from GitHub's own compare, then uploads the artifacts. Running it twice
replaces the assets instead of failing, so a re-run after a failed upload is safe.

Usage:
  GITHUB_TOKEN=... GITHUB_REPO=radaiko/SnackPilot TAG=desktop/v2.2.0 \
  TITLE="SnackPilot Desktop v2.2.0" BODY_FILE=notes.md \
  python3 tools/ci/github_release.py dist/macos/*.zip dist/windows/*.exe

Never prints the token. GitHub's API answers with 4xx and a JSON message on
failure, which is raised as an error with the message intact.
"""
import json
import os
import sys
import urllib.error
import urllib.parse
import urllib.request

API = "https://api.github.com"
UPLOADS = "https://uploads.github.com"


def call(url, method="GET", body=None, headers=None, raw=None):
    data = raw if raw is not None else (json.dumps(body).encode() if body is not None else None)
    req = urllib.request.Request(url, data=data, method=method, headers=headers or {})
    req.add_header("Authorization", f"Bearer {os.environ['GITHUB_TOKEN']}")
    req.add_header("Accept", "application/vnd.github+json")
    req.add_header("User-Agent", "snackpilot-desktop-release")
    if body is not None:
        req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req, timeout=120) as res:
            payload = res.read()
    except urllib.error.HTTPError as error:
        detail = error.read().decode("utf-8", "replace")
        raise SystemExit(f"github: {method} {url} -> {error.code}\n{detail}") from error
    return json.loads(payload) if payload else None


def release_for(repo, tag):
    try:
        return call(f"{API}/repos/{repo}/releases/tags/{urllib.parse.quote(tag, safe='')}")
    except SystemExit:
        return None


def main():
    repo = os.environ.get("GITHUB_REPO", "radaiko/SnackPilot")
    tag = os.environ["TAG"]
    title = os.environ.get("TITLE", tag)
    target = os.environ.get("TARGET_COMMITISH", "main")
    body_file = os.environ.get("BODY_FILE")
    body = open(body_file, encoding="utf-8").read() if body_file else ""

    assets = sys.argv[1:]
    missing = [path for path in assets if not os.path.isfile(path)]
    if missing:
        raise SystemExit(f"missing artifacts: {', '.join(missing)}")
    if not assets:
        raise SystemExit("no artifacts given")

    release = release_for(repo, tag)
    if release is None:
        release = call(
            f"{API}/repos/{repo}/releases",
            method="POST",
            body={
                "tag_name": tag,
                "target_commitish": target,
                "name": title,
                "body": body,
                "draft": False,
                "prerelease": False,
                # Notes from GitHub's compare, appended to whatever body we send.
                "generate_release_notes": True,
            },
        )
        print(f"github: created {release['html_url']}")
    else:
        # A re-run must also refresh the notes: the trust state in them comes
        # from the build, so a signed run replacing an unsigned one has to say
        # so. Assets are replaced below by name.
        if body:
            release = call(
                f"{API}/repos/{repo}/releases/{release['id']}",
                method="PATCH",
                body={"name": title, "body": body},
            )
        print(f"github: release for {tag} exists ({release['html_url']}), notes and assets updated")

    existing = {asset["name"]: asset for asset in release.get("assets", [])}
    for path in assets:
        name = os.path.basename(path)
        stale = existing.get(name)
        if stale:
            call(f"{API}/repos/{repo}/releases/assets/{stale['id']}", method="DELETE")
        with open(path, "rb") as handle:
            payload = handle.read()
        query = urllib.parse.urlencode({"name": name})
        asset = call(
            f"{UPLOADS}/repos/{repo}/releases/{release['id']}/assets?{query}",
            method="POST",
            headers={"Content-Type": "application/octet-stream"},
            raw=payload,
        )
        print(f"github: uploaded {name} ({asset['size']} bytes)")

    print(f"github: published {release['html_url']}")


if __name__ == "__main__":
    main()
