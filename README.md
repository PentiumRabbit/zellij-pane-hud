# zellij-tree-hud

A HUD-style finder for Zellij sessions, tabs and panes.

The project is currently in the design and research phase. No plugin
implementation exists yet.

## Goal

Press a configurable shortcut to open a floating search window, browse a tree
of:

```text
session
└── tab
    ├── pane  running command
    └── pane  shell
```

Type to fuzzy-filter the tree, move with the keyboard, and press Enter to jump
directly to the selected terminal pane. Cross-session pane jumps should switch
to the target session and focus the pane in one operation.

The visual language should follow `zellij-hud`: compact, low-chrome, Nerd Font
icons, and useful status cues without a permanent sidebar.

## Desired defaults

- Search scope: all live sessions plus resurrectable sessions.
- Tree levels: session, tab and pane.
- Current session: live command and pane-state icons.
- Other live sessions: structural tree and best-effort status.
- Resurrectable sessions: leaf nodes because their tabs and panes are not live.
- Enter on a session focuses that session.
- Enter on a tab focuses that tab.
- Enter on a pane switches to the session, tab and pane.
- Esc closes the popup.
- Up/Down, `Ctrl+p`/`Ctrl+n`, Backspace and direct typing operate inside the
  finder.

All three tree levels are intended to be configurable.

## Shortcut

The requested sequence is `Ctrl+Space`, then `Space`. Zellij 0.45.1 does not
allow user-defined input modes, so this sequence needs a small background
launcher and a timeout. A single `Ctrl+Space` remains the simplest fallback.

See [Research](docs/RESEARCH.md) for the tested constraint.

## Documentation

- [Design proposal](docs/DESIGN.md)
- [Technical research](docs/RESEARCH.md)
- [Prototype fixtures](prototypes/README.md)

## Status

- [x] Version-specific API research
- [x] Existing-plugin comparison
- [x] Initial UX and architecture proposal
- [ ] User review of the design
- [ ] Implementation plan
- [ ] Rust/WASM implementation
