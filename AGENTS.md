# SnackPilot - agent instructions

Cafeteria ordering and billing: one gpui-kit UI in Rust (`src/ui`) for iOS, Android
and a desktop development window, over a shared Rust core (`src/core`, UniFFI) that
owns scraping, parsing, caching, fingerprinting and notification decisions. There are
no first-party servers; the app scrapes Gourmet and Ventopay directly.

## Source of truth

`docs/requirements/` is authoritative, extracted and verified from v1.4.5. When code
and docs disagree, treat it as a defect in one of them and resolve it explicitly.

**Never deviate from the scraping specs** (`01-gourmet-scraping.md`,
`02-ventopay-scraping.md`): the exact request sequences, headers, encodings and
parameter values are account-ban-critical.

The README keeps the credit link to `github.com/patrickl92/GourmetClient`. The bundle
ID and package `dev.radaiko.gourmetclient` must not change.

## Commands

```sh
make check      # everything CI runs: fmt, clippy, core tests, spec validation
make lint       # cargo fmt --check + clippy
make test       # cargo test for the workspace
make spec-lint  # openspec validate --all --strict --no-interactive
make doctor     # required build tools
```

Run `make check` before claiming done. Do not invent alternatives.

## The loop

Every feature starts as an OpenSpec change written by `spec` with the human. The change
declares its domain (`core`, `ui`, `architectural`) and every task its `tier`, `files`,
`deps`, `reqs` and done bar. A goal session then runs the `dispatch` skill: one `coder`
subagent per task, `make check`, one `review`, merge, tick.

| Step | Agent | Model |
| --- | --- | --- |
| Spec | `spec` | DeepSeek V4.1 Flash; UI: GLM-5.3; architectural: Sonnet 5.5 · medium |
| Dispatch | goal session + `dispatch` skill | local Qwen 3.8 Flash-Next, thinking off |
| Code | `coder` | local Qwen 3.8 Flash-Next, thinking medium |
| Review | `review` | Sonnet 5.5 · low |

- Integration branch is `main`; one task branch per change, merged when `make check` is
  green and the review is `PASS`.
- Two fix rounds per task, counting a red check and review findings together. Then the
  task stops and goes to the human.
- One task at a time; no worktrees.
- The dispatcher runs `make check`; the reviewer gets that output and does not re-run it.

## Conventions

- Rust (edition in `src/core/Cargo.toml`); core tests under `src/core/tests`.
- UI workspace `src/ui`: model tests under `src/ui/model/tests`, screens in
  `src/ui/app/src/screens`, hosts in `src/ui/{ios,android}`; `make ui-check` in `make check`.
- Some odd behaviour is deliberate v1 parity: persist-before-validate, logout keeps
  credentials, cookies retained across logout, device-local midnight for the
  upcoming/past split, `de-AT` formatting, the 200-entry notification log cap. Check
  `docs/requirements/` before calling one a bug.
- Fixtures in `docs/fixtures/` and `src/core/tests/fixtures/` must match by hash;
  `build.rs` re-validates at build time.
- Secrets live in Proton Pass, never in the repository or a prompt.

## Definition of done

- [ ] `make check` passes locally
- [ ] every changed requirement has a test tagged `@req REQ-...`
- [ ] the review gate passed, and its verdict is quoted in the handoff
- [ ] every defect a review found is fixed with a test that fails without the fix
- [ ] any new dependency is named in the task's `deps:` line
- [ ] the diff stays within the task's stated files
- [ ] the PR states the task id, the REQs covered and the `make check` output

## Boundaries

Never, without explicit approval in the conversation:

- edit `docs/requirements/`, `openspec/specs/`, CI config, or dependency manifests
  (`Cargo.toml`, `Cargo.lock`)
- change a Gourmet or Ventopay request shape
- force-push, rewrite shared history, or push to `main` outside the loop's merge
- read or print secrets, credentials or cookie jars
- put real credentials or live responses in the repository or in a prompt

## Threat model

In scope: malformed HTML or JSON, a changed response shape, a failing network, an
expired or rotated credential, a double tap on an order action. Out of scope: malicious
third-party sites beyond what the scraping specs describe. Always in scope: the request
shapes stay exactly as specified, and no credential appears in a log, error or fixture.

## Review checklist

- Every changed requirement has an `@req` test, and the tag names a real requirement.
- The Gourmet and Ventopay request shapes match `docs/requirements/01` and `02`.
- Fixtures are sanitized and match their mirrors; no live data.
- No credential, cookie, token or response body in a log, error message or fixture.
- Tests do not reach a real host.
