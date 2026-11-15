# Prototype fixtures

## `ctrl-space-custom-mode.failed.kdl`

This fixture tests whether Zellij 0.45.1 can implement a two-step shortcut with
a user-defined input mode:

```text
Ctrl+Space -> Space
```

It is expected to fail validation:

```text
Unknown InputMode 'pane-hud-leader'
```

The failure is intentional research evidence. Do not use this file as a
production configuration.
