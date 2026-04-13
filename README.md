# zj-hooker


This is a plugin for [zellij <3](https://github.com/zellij-org/zellij) that allows you to run
commands on session attach and detach.

https://github.com/user-attachments/assets/e273f850-9d3c-4aa3-81fa-855f02399d9c

## Example configuration
```kdl
# ~/.config/zellij/config.kdl
...
plugins {
...
    zj-hooker location="file:~/.config/zellij/plugins/zj-hooker.wasm" {
        pattern "main"                # exact match
        pattern "dev*"                # prefix match
        pattern "*dev"                # suffix match
        pattern "*dev*"               # contains match
        on_attach "echo 'attached'"   # or "file:~/on_attach.sh"
        on_detach "echo 'detached'"   # or "file:~/on_detach.sh"
        attach_mode "interactive"     # optional: "interactive/background(default)"
    }
...
}

# my-layout.kdl
layout {
    default_tab_template {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        children
        pane size=2 borderless=true {
            plugin location="zellij:status-bar"
        }
    }

    tab name="zellij <3" cwd="~/" {
        floating_panes {
            pane cwd="~/" {
                x "50%"
                y "50%"
                width "50%"
                height "50%"
                plugin location="zj-hooker"
            }
        }
        pane
    }
}

```

Now when you attach to a session, the plugin will run the command/script specified for on_attach key.
Same for on_detach key.

Note: if you quit session no command or script will run: as far as  I learned, this is a fundamental limitation of
zellij.

## Interactive vs Background

Note: on_attach runs in "background" mode by default (non-interactive)
      Use "attach_mode" "interactive" to open a floating pane (supports sudo, etc.)
      If command contains "sudo", automatically uses interactive mode
      Use "file:/path/to/script.sh" to run a script file
