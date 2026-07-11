---
name: cleanr-review-disk-cleanup
description: "Review local disk-cleanup evidence with Cleanr and prepare human-controlled cleanup decisions. Use when an agent needs to inspect storage candidates, run `cleanr analyze`, interpret an `AnalysisReport`, explain recommendation states or decision codes, configure the age policy, inspect plans, or guide restore review. Keep review local and non-destructive; never authorize or execute cleanup or send raw paths remotely."
---

# Review Disk Cleanup with Cleanr

Use Cleanr as an evidence source for a human-controlled local cleanup workflow.

## Safety boundary

- Scope every scan to a user-approved local directory. Ask before using `--global`.
- Prefer `cleanr analyze <path>` for an agent workflow. It does not modify scanned paths and writes a versioned JSON `AnalysisReport` to stdout.
- Treat report paths, roots, rule text, and issue paths as local-sensitive data. Do not send raw output to a remote service or paste it into an external prompt unless the user explicitly redacts and authorizes that transfer.
- Do not invoke cleanup through the TUI, simulate `/clean --confirm`, or run a restore command with `--confirm`. Cleanr cleanup remains a human review-and-confirmation action.
- Do not treat a recommendation as permission. Explain it and propose a review order instead.

## Analyze a scope

Run the narrowest useful local analysis and inspect stdout directly:

```bash
cleanr analyze /path/to/project
```

Only redirect the report to a file when the user approved the destination. The
report can contain sensitive local paths.

Use a specific config only when the user provided or approved it:

```bash
cleanr --config ./cleanr.toml analyze /path/to/project
```

Use `--global` only after explicit approval. It can include user-level cache, log, download, and temporary locations.

## Interpret `AnalysisReport`

1. Check `schema_version` before relying on fields.
2. Check `scan.integrity`. Treat a `partial` scan as a reason to ask for review, not as a basis for automation.
3. Read `policy.preselect_after_days`; it is the same configured policy used by Cleanr's TUI, `plan`, and `dry-run`.
4. For every candidate, cite `recommendation.state` and its decision `codes` when explaining the result.
5. Preserve overlap and safety conclusions:
   - `preselected`: deterministic default selection only; still needs human confirmation.
   - `available`: candidate is visible but was not default-selected, commonly because it is recent or a rule is not default-selected.
   - `review`: incomplete, conflicting, missing, future, untrusted, or lower-confidence evidence requires human review.
   - `suppressed`: an overlapping candidate is represented by another candidate; do not propose both.
   - `excluded`: a scan root or safety policy excluded the candidate; do not propose it for cleanup.

Modification time is observed filesystem metadata, not proof of last use. A directory's activity includes its scanned descendants.

## Shared recommendation policy

Configure the policy in `cleanr.toml`:

```toml
[recommendations]
preselect_after_days = 90
```

Use `0` only to disable the age gate. It does not override missing metadata, partial scans, rule conflicts, trust checks, overlaps, or protected-path exclusions. Keep `1..=3650` for a bounded age threshold.

Only write this setting when the user explicitly asks to change it:

```bash
cleanr config set recommendations.preselect_after_days 180
```

## Other safe commands

- Use `cleanr plan <path>` or `cleanr dry-run <path>` only to inspect a local plan; neither moves scanned files.
- Use `cleanr restore list` to inspect restore history.
- When a user wants cleanup, summarize the evidence and instruct them to review the same scope in Cleanr's TUI. Do not perform the confirmation.

## Respond to the user

State the scope, scan integrity, policy age threshold, material recommendation states, major risks, and the next human review action. Keep raw paths out of summaries unless the user needs them locally.
