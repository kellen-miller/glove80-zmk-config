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
