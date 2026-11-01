#!/usr/bin/env python3
"""One-way sync of GitHub issues and comments into Forgejo.

Copies are found by a hidden marker in their body, so the script keeps no state and can always run again.
GitHub is read without a token because the repository is public.
"""
import json
import os
import re
import urllib.parse
import urllib.request

GITHUB_API = "https://api.github.com"
FORGEJO_API = os.environ["FORGEJO_API"].rstrip("/")
FORGEJO_REPO = os.environ["FORGEJO_REPO"]
FORGEJO_TOKEN = os.environ["FORGEJO_TOKEN"]
GITHUB_REPO = os.environ["GITHUB_REPO"]
ISSUE_MARKER = re.compile(r"<!-- github-issue:(\d+) -->")
COMMENT_MARKER = re.compile(r"<!-- github-comment:(\d+) -->")


def call(url, method="GET", body=None, headers=None):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(url, data=data, method=method, headers=headers or {})
    req.add_header("Accept", "application/json")
    if data:
        req.add_header("Content-Type", "application/json")
    with urllib.request.urlopen(req, timeout=30) as res:
        raw = res.read()
    return json.loads(raw) if raw else None


def github(path):
    items, page = [], 1
    while True:
        batch = call(f"{GITHUB_API}/repos/{GITHUB_REPO}/{path}{'&' if '?' in path else '?'}per_page=100&page={page}",
                     headers={"User-Agent": "issue-sync", "Accept": "application/vnd.github+json"})
        items += batch
        if len(batch) < 100:
            return items
        page += 1


def forgejo(path, method="GET", body=None):
    return call(f"{FORGEJO_API}/repos/{FORGEJO_REPO}/{path}", method, body,
                {"Authorization": f"token {FORGEJO_TOKEN}"})


def forgejo_all(path):
    items, page = [], 1
    while True:
        sep = "&" if "?" in path else "?"
        batch = forgejo(f"{path}{sep}limit=50&page={page}")
        items += batch
        if not batch:
            return items
        page += 1


def issue_body(issue):
    head = f"<!-- github-issue:{issue['number']} -->\nMirrored from github#{issue['number']} " \
           f"({issue['html_url']}), opened by @{issue['user']['login']}.\n\n"
    return head + (issue.get("body") or "")


def comment_body(comment):
    head = f"<!-- github-comment:{comment['id']} -->\n@{comment['user']['login']} on GitHub ({comment['html_url']}):\n\n"
    return head + (comment.get("body") or "")


def sync_comments(issue, copy_number):
    existing = {}
    for comment in forgejo_all(f"issues/{copy_number}/comments"):
        match = COMMENT_MARKER.search(comment.get("body") or "")
        if match:
            existing[int(match.group(1))] = comment
    created = 0
    for comment in github(f"issues/{issue['number']}/comments"):
        body = comment_body(comment)
        copy = existing.get(comment["id"])
        if copy is None:
            forgejo(f"issues/{copy_number}/comments", "POST", {"body": body})
            created += 1
        elif copy["body"] != body:
            forgejo(f"issues/comments/{copy['id']}", "PATCH", {"body": body})
    return created


def main():
    copies = {}
    for copy in forgejo_all("issues?state=all&type=issues"):
        match = ISSUE_MARKER.search(copy.get("body") or "")
        if match:
            copies[int(match.group(1))] = copy
    created = updated = comments = 0
    for issue in github("issues?state=all"):
        if "pull_request" in issue:
            continue
        body, copy = issue_body(issue), copies.get(issue["number"])
        if copy is None:
            copy = forgejo("issues", "POST", {"title": issue["title"], "body": body})
            created += 1
        changes = {}
        if copy["title"] != issue["title"]:
            changes["title"] = issue["title"]
        if copy["body"] != body:
            changes["body"] = body
        if copy["state"] != issue["state"]:
            changes["state"] = issue["state"]
        if changes:
            forgejo(f"issues/{copy['number']}", "PATCH", changes)
            updated += 1
        if issue["comments"] or copy.get("comments"):
            comments += sync_comments(issue, copy["number"])
    print(f"created {created}, updated {updated}, new comments {comments}")


if __name__ == "__main__":
    main()
