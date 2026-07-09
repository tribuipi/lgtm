# review

A fast, native code-review app in Rust on [gpui](https://www.gpui.rs/).
See [DESIGN.md](DESIGN.md) for the full design.

## Usage

Requires the [GitHub CLI](https://cli.github.com/) (`gh auth login` first).

```
review owner/repo#123
review 123                      # repo inferred from the cwd's git remote
review https://github.com/owner/repo/pull/123
```

## Keys

| Key | Action |
|---|---|
| `]` / `[` | next / previous file |
| `n` / `p` | next / previous hunk |
| `home` / `end` | top / bottom |
| `cmd-q` | quit |

## Status

Milestone 1 (walking skeleton): unified diff view with add/remove tints,
word-level intra-line highlights, gutter line numbers, keyboard nav.
Next: tree-sitter syntax highlighting, split view, file tree, selection,
Helix theme loading — see the milestones in DESIGN.md.
