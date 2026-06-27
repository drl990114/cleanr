---
sidebar_position: 4
description: Learn what Cleanr protects, what cleanup changes, and when restore can or cannot succeed.
---

# Safety and recovery

Cleanr is designed to make cleanup reviewable and reversible, but "moved to
trash" is not the same as a permanent backup. This page defines the boundary.

## What happens during cleanup

For every selected item, Cleanr:

1. creates a pending execution manifest for the selected items;
2. verifies that the path is inside a scanned root;
3. rejects filesystem roots, symbolic links, and protected Cleanr paths;
4. checks that the item type, modification time, file size, and directory
   fingerprint still match the scan where applicable;
5. moves the item to the operating system trash;
6. records the result and restore locator back into the execution manifest.

Validation happens immediately before each item is moved. If a target changed
after the scan, that item fails safely and remains in place.

## Protected paths

Cleanr excludes:

- your home directory as a cleanup target;
- the active Cleanr executable and configuration file;
- Cleanr's state directory, including cleanup and restore history;
- configured plugin and language directories.

The contents below your home directory can still be scanned. The protection
prevents the home directory itself, or another protected subtree, from being
selected as one cleanup item.

## How selection works

An item is preselected only when all of the following are true:

- the rule confidence is `High`;
- the rule declares `default_selected = true`;
- the rule comes from Cleanr itself or an explicitly trusted plugin.

Everything can still be deselected before cleanup. General downloads, logs,
temporary files, medium-confidence items, and untrusted plugin matches require
manual selection.

## Manifests and history

Cleanr stores:

- an execution manifest for every cleanup run;
- a restore manifest for every restore attempt;
- per-item success, failure, and rollback information.

These files live under the platform state directory in a `cleanr` folder.
They are required for Cleanr's restore history, so do not delete that directory
if you may need to undo a cleanup.

## Restore support

Programmatic restore is implemented for:

- macOS Trash;
- Windows Recycle Bin;
- Linux desktops with Freedesktop-compatible trash support.

Restore is best-effort. It cannot recover an item after the system trash has
been emptied, and it will not overwrite a path that has been recreated.
External tools that alter trash metadata can also make matching impossible.

If Cleanr cannot restore an item, inspect the system trash and the manifest
before taking further action.

## Confirmation and agents

Cleanup requires a local user authorization token in the execution layer.
Model-generated actions cannot create that token.

Setting `cleanup.require_confirm = false` removes the interactive confirmation
dialog for a direct local `/clean` request, but it does not grant an agent
permission to execute cleanup.

## Practical safety checklist

- Start with a narrow project directory.
- Read the risk note for unfamiliar candidates.
- Keep source files and irreplaceable data in version control or backups.
- Do not empty the system trash until you are confident the cleanup was
  correct.
- Keep Cleanr's state directory while you still need restore history.
