## Summary

```text
stable Engrammer / Enthium releases
  → pinned source commits
  → macOS JSON + generated firmware
  → firmware validation + human review
```

Exact versions and commits: `upstream-lock.json`. Existing local customization
must pass locked regeneration before an update can be generated.

## Evidence

- **Before:** previous versions in the base branch's `upstream-lock.json`.
  **After:** updater regenerated matching JSON bindings and firmware snippets;
  PR validation checks reproducibility and the independent Enthium update path.
- Firmware Build results appear in this PR. Review them before merging/flashing.

## Merge Danger

**Door:** two-way

Revert this PR to restore the previous keymap. No automatic flashing or merging.

**Blast Radius:** keyboard
