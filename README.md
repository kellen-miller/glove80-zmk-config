# Glove80: Enthium v14 for macOS

Full [Glorious Engrammer v52](https://github.com/sunaku/glove80-keymaps/releases/tag/v52)
with [Enthium v14](https://github.com/sunaku/enthium/releases/tag/v14), configured
for macOS. Replaces the previous four-layer custom layout.

Imported from `sunaku/glove80-keymaps` commit
`d857d0a5fec0883e721230bac27db2fc9f81c93e` (v52 plus two fixes).

- Base layout: Enthium v14, including right-thumb R and left-thumb Space.
- macOS shortcuts and CAGS home-row modifiers: Control, Option, Command, Shift
  from pinky to index, mirrored on the right hand.
- Upstream timing defaults; bilateral home-row modifiers enabled.
- All 32 upstream layers, including Dvorak, Colemak, QWERTY, and Factory.
- Mouse keys enabled; standard RGB support. Per-key layer RGB is optional and
  requires the compatible firmware and configuration described upstream.
- Shift, space, and thumb forgiveness and natural scrolling remain disabled.

The [upstream guide](https://github.com/sunaku/glove80-keymaps/tree/d857d0a5fec0883e721230bac27db2fc9f81c93e#guide)
explains the layers, thumb combinations, and timing options. The
[layer diagrams](https://github.com/sunaku/glove80-keymaps/blob/d857d0a5fec0883e721230bac27db2fc9f81c93e/README/all-layer-diagrams.pdf)
show every key. Hold Magic and tap the left thumb T3 key to toggle Factory.

`config/glove80.keymap` is the firmware build input. `config/glove80.conf` enables
pointing support. `config/keymap.json` is the matching MoErgo Layout Editor
export; keep its bindings and custom behaviors synchronized with the keymap.
`config/info.json` describes physical key positions, not the active layout.

Push a branch or open a pull request to run the existing **Build** workflow.
It builds both halves against `moergo-sc/zmk` main, combines their firmware,
and uploads the `glove80.uf2` artifact. A passing build does not flash the keyboard.

For a local build, clone `moergo-sc/zmk` into `src` and run
`nix-build config -o combined`; the output is `combined/glove80.uf2`.

Download the build artifact, then follow
[MoErgo's flashing instructions](https://docs.moergo.com/glove80-user-guide/customizing-key-layout/#loading-new-zmk-firmware-onto-your-glove80).
When changing firmware versions, flash both halves and follow MoErgo's
configuration reset and re-pairing instructions. Choose a short filename such
as `glove80.uf2` when copying firmware from macOS.

Unicode/Emoji macros on macOS need the Unicode Hex Input source described in
the upstream guide; ordinary typing and shortcuts use your normal US input source.

The imported keymap is covered by [Sunaku's ISC license](config/LICENSE.sunaku);
the original repository files retain their existing license.

## Automatic upstream updates

A Rust maintenance job polls stable GitHub releases on the first of each month at 13:23 UTC, or on
manual dispatch. Cross-repository releases cannot directly trigger this repo's
`release` event. Drafts/prereleases and non-`v<number>` tags are excluded; a
release older than the lock fails instead of downgrading. With no new release,
the job makes no changes.

`upstream-lock.json` records both versions and exact commits. The initial
Engrammer pin intentionally retains v52's two imported fixes. It moves to the
release commit only when a newer stable Engrammer version appears.

When either project releases, the updater imports the full Engrammer JSON,
projects Enthium's released XKB letter/punctuation/thumb mapping onto its base
layer, then runs upstream's Ruby ERB generation (`rake -B keymap.dtsi`). That
regenerates layout-dependent home-row/combo definitions before rendering all
firmware layer bindings. The macOS override, pointing configuration, and local
creator/privacy metadata survive. The updater first verifies the previous lock
against local JSON and firmware; custom edits fail for manual reconciliation
rather than being overwritten. Unsupported upstream schemas, keysyms, export
structures, or configuration changes also fail visibly.

The maintenance workflow separates read-only generation from publishing.
Configure repository Actions secret **`UPSTREAM_UPDATE_TOKEN`** with a
fine-grained PAT scoped to this repository: **Contents: read/write** and
**Pull requests: read/write**. Its owner must have repository access. This
credential is used only in the publisher job, never passed to upstream Ruby.
The default `GITHUB_TOKEN` cannot trigger normal PR validation for PRs it
creates; this separate token lets the existing firmware Build run on the bot PR.
No token is committed here. Configure/rotate the secret before expecting new
release PRs to publish.

The bot owns `chore/upstream-keymap-releases`: do not edit that branch manually.
It maintains one review PR and replaces its generated outputs when newer
releases arrive. Identical pending outputs are skipped, avoiding repeated firmware
rebuilds. Merging and flashing remain manual. Schedule activation requires this
workflow on the default branch; run failures appear in Actions.

Local requirements: Rust (edition 2024), Ruby, Rake, Git, and GitHub CLI
(authenticated for API release queries). From the repository root:

```sh
cargo test --locked --manifest-path updater/Cargo.toml
cargo run --locked --manifest-path updater/Cargo.toml -- check
cargo run --locked --manifest-path updater/Cargo.toml -- update
```

`check` fetches immutable source commits, verifies existing JSON and firmware
(comment/whitespace differences ignored in generated snippets), then tests an
independent Enthium letter change through the actual upstream generator. It
writes only temporary files. `update` writes the two synchronized config files
and lock only after successful generation; review their diff and Build results.
The introductory version links above describe the original import; the lock is
the authoritative version record after automated updates.
