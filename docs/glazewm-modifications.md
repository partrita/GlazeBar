# GlazeWM modifications

This is a modified build of [glzr-io/glazewm](https://github.com/glzr-io/glazewm), based on v3.10.1 and built as `3.10.1-glazebar.2`. It is not an upstream release. GlazeWM keeps its upstream GPL-3.0 license; GlazeBar is a separate component.

The patch against the upstream v3.10.1 source is [glazewm-v3.10.1.patch](../glazewm-patches/glazewm-v3.10.1.patch). Every x64 and ARM64 bundle includes the matching modified source, the patch and these notes.

## What changed

- **Minimized groups.** `wm-toggle-minimized-group` / `wm-restore-minimized-group` natively minimize every window of the current workspace and restore them later. The WM captures the workspace, split identifiers, directions, proportions and focus order, so nothing is rearranged and no workspace switching is involved. Windows closed or moved in between are left out on restore; newly opened windows stay managed. The snapshot lives in memory for the running WM session.
- **Faster restore.** Window types go straight back into the saved tree, skipping the per-window insertion heuristics. Windows are restored to their final rectangle in one step instead of `SW_RESTORE` followed by another placement. Redundant shell cloaking calls are skipped, and focus is applied once after the group is back. Group placement uses `SW_SHOWNOACTIVATE`, so restored windows don't each grab activation. The final screen-coordinate `SetWindowPos` correction stays, because removing it made some windows overlap.
- **Rapid toggles.** Automatic split changes are suppressed for two seconds around native minimize/restore, and late minimize notifications from a quick second toggle are ignored during that window.
- **`wm-redraw` and empty splits.** Empty nested splits are removed and their space goes back to the surviving siblings. The same cleanup runs before every redraw during platform sync.
- **Focus.** No synthetic foreground input is sent when the target already owns foreground focus. The WM compares the actual OS focus with its own focus and observes foreground events instead of waiting synchronously for an activation.
- **GlazeBar integration.** GlazeBar calls the group command on the modified WM and falls back to its previous behaviour on upstream builds.
- **Upstream test fixture.** A 600-pixel window with one pixel visible at the left edge starts at x = -599, not +1; the corner-position unit test now says so.

## preview.7 fix

The preview.4–6 builds could end up with a broken container tree after a window was moved. The WM then logged `No workspace is currently focused.`, `Focused container has no monitor.` and `No workspace.` over and over. With `focus_follows_cursor` enabled, the monitor error repeated on every mouse move, and in the session where this was found the WM's IPC stopped answering.

**Cause.** `detach_container` recursively detached any ancestor split that became empty. `move_container_within_tree` detaches a container and then attaches it to a target parent chosen beforehand. When that target was an emptied ancestor, the window was attached to a split that had just been removed from the tree, so its ancestor chain no longer reached a workspace or a monitor.

**Fix.**

- `detach_container` leaves emptied ancestor splits attached, as upstream does.
- `platform_sync` runs `prune_empty_splits` before it redraws. Empty splits are still cleaned up, at a point where no caller holds a reference to one.
- Regression test `detach_keeps_emptied_ancestor_split_attached` fails with the recursive detach (the ancestor's parent becomes `None`) and passes without it.

The full `move_container_within_tree` path needs a `WmState` and is not covered by a unit test.

One related limitation: the minimized-group snapshot is global rather than per workspace. Toggling from a different workspace restores the original workspace's layout and gives focus to it even if it isn't displayed.

## Building

Use Rust nightly (tested: `1.101.0-nightly`, `b6eeb93c9`, 2026-10-18), the MSVC tools and Windows SDK for the target architecture, and set `VERSION_NUMBER=3.10.1-glazebar.2`:

```
cargo +nightly build --release --offline --locked -p wm --bin glazewm -p wm-watcher --bin glazewm-watcher
```

For ARM64 use the `vcvarsamd64_arm64` environment and add `--target aarch64-pc-windows-msvc`. Extract `glazewm-modified-source.zip` into a build folder, then extract `glazewm-vendor.zip` into its `vendor/` subfolder next to `Cargo.toml`. The patch is applied to the upstream v3.10.1 source; it contains no vendored dependencies.

The modified WM and watcher are installed in `%USERPROFILE%/.glzr/glazewm/wm`, and both the Start-menu and optional login shortcuts run them. The official MSI stays installed for CLI compatibility and rollback; avoid enabling a separate upstream autostart entry.

## Validation

- Unit tests: 5 passed, including tree restoration, nested empty-split removal and the preview.7 regression test.
- Group minimize/restore on a real x64 system with 7–8 windows, three rounds: native `IsIconic`, displayed workspaces, split IDs, proportions and window rectangles all matched before and after. Five rapid double toggles preserved the tree.
- Restore time with 6–8 windows was 350–830 ms, depending on window count and which restore optimizations were in place; minimize took 58–91 ms in the 6-window test. Direct WM command responses were 1–14 ms, which doesn't include the asynchronous native window transitions. Disabling per-window activation and the minimize/maximize animation setting didn't remove the remaining delay.
- preview.7: WM rebuilt, installed and running on x64 after the fix; no new errors in `errors.log` after the restart.
- ARM64 binaries are cross-built on x64 and checked as packages; they haven't been run on physical ARM64 hardware.
