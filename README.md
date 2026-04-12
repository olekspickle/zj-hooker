# zj-hooker


This is a plugin for [zellij](https://github.com/zellij-org/zellij) that allows you to run
commands on session attach and detach.

## Example configuration
```kdl
zj-hooker location="file:~/.config/zellij/plugins/zj-hooker.wasm" {
    "pattern" "main"                # exact match
    "pattern" "dev*"                # prefix match
    "pattern" "*dev"                # suffix match
    "pattern" "*dev*"               # contains match
    "on_attach" "echo 'attached'"   # or "file:~/script.sh"
    "on_detach" "echo 'detached'"   # or "file:~/script.sh"
    "attach_mode" "interactive"     # optional: "interactive/background(default)"
}
```

## Interactive vs Background

Note: on_attach runs in "background" mode by default (non-interactive)
      Use "attach_mode" "interactive" to open a floating pane (supports sudo, etc.)
      If command contains "sudo", automatically uses interactive mode
      Use "file:/path/to/script.sh" to run a script file
