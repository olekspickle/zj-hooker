//!
//! This is a plugin for [zellij <3](https://github.com/zellij-org/zellij) that allows you to run
//! commands on session attach and detach.
//!
//! # Example configuration
//! ```kdl
//! # ~/.config/zellij/config.kdl
//! ...
//! plugins {
//! ...
//!     zj-hooker location="file:~/.config/zellij/plugins/zj-hooker.wasm" {
//!         "pattern" "main"                # exact match
//!         "pattern" "dev*"                # prefix match
//!         "pattern" "*dev"                # suffix match
//!         "pattern" "*dev*"               # contains match
//!         "on_attach" "echo 'attached'"   # or "file:~/on_attach.sh"
//!         "on_detach" "echo 'detached'"   # or "file:~/on_detach.sh"
//!         "attach_mode" "interactive"     # optional: "interactive/background(default)"
//!     }
//! ...
//! }
//!
//! # my-layout.kdl
//! layout {
//!     default_tab_template {
//!         pane size=1 borderless=true {
//!             plugin location="zellij:tab-bar"
//!         }
//!         children
//!         pane size=2 borderless=true {
//!             plugin location="zellij:status-bar"
//!         }
//!     }
//!
//!     tab name="zellij <3" cwd="~/" {
//!         floating_panes {
//!             pane cwd="~/" {
//!                 x "50%"
//!                 y "50%"
//!                 width "50%"
//!                 height "50%"
//!                 plugin location="zj-hooker"
//!             }
//!         }
//!         pane
//!     }
//! }
//!
//! ```
//!
//! Now when you attach to a session, the plugin will run the command/script specified for on_attach key.
//! Same for on_detach key.
//!
//! Note: if you quit session no command or script will run: as far as  I learned, this is a fundamental limitation of
//! zellij.
//!
//! # Interactive vs Background
//!
//! Note: on_attach runs in "background" mode by default (non-interactive)
//!       Use "attach_mode" "interactive" to open a floating pane (supports sudo, etc.)
//!       If command contains "sudo", automatically uses interactive mode
//!       Use "file:/path/to/script.sh" to run a script file

use std::collections::BTreeMap;
use std::path::PathBuf;

use zellij_tile::prelude::*;

static ON_ATTACH_KEY: &str = "on_attach";
static ON_DETACH_KEY: &str = "on_detach";
static PATTERN_KEY: &str = "pattern";
static ATTACH_MODE_KEY: &str = "attach_mode";

#[derive(Default)]
struct State {
    is_attached: bool,
    on_attach: Option<String>,
    on_detach: Option<String>,
    pattern: Option<String>,
    attach_mode: AttachMode,
    attach_pane_id: Option<u32>,
    pane_manifest: Option<PaneManifest>,
}

#[derive(Default, Clone, Copy, PartialEq, Debug)]
enum AttachMode {
    #[default]
    Background,
    Interactive,
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        request_permission(&[
            PermissionType::RunCommands,
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);

        self.on_attach = configuration.get(ON_ATTACH_KEY).cloned();
        self.on_detach = configuration.get(ON_DETACH_KEY).cloned();
        self.pattern = configuration.get(PATTERN_KEY).cloned();

        if let Some(mode) = configuration.get(ATTACH_MODE_KEY) {
            self.attach_mode = match mode.as_str() {
                "interactive" => AttachMode::Interactive,
                _ => AttachMode::Background,
            };
        }

        subscribe(&[
            EventType::SessionUpdate,
            EventType::PaneUpdate,
            EventType::RunCommandResult,
            EventType::BeforeClose,
        ]);
        eprintln!(
            "[zj-hooker] loaded: on_attach={:?}, on_detach={:?}, pattern={:?}, attach_mode={:?}",
            self.on_attach, self.on_detach, self.pattern, self.attach_mode
        );
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::SessionUpdate(sessions, _) => {
                if let Some(session) = sessions.iter().find(|s| s.is_current_session) {
                    let client_count = session.connected_clients;
                    match (client_count, self.is_attached) {
                        (0, true) => {
                            self.is_attached = false;
                            if let Some(cmd) = self.on_detach.clone() {
                                self.run_command_detach(&cmd);
                            }
                            self.attach_pane_id = None;
                        }
                        (n, false) if n > 0 => {
                            self.is_attached = true;
                            self.attach_pane_id = None;
                            if let Some(ref pattern) = self.pattern
                                && self.matches_pattern(&session.name, pattern)
                                && let Some(cmd) = self.on_attach.clone()
                            {
                                self.run_command_attach(&cmd);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Event::PaneUpdate(manifest) => {
                self.pane_manifest = Some(manifest.clone());
                if let Some(ref attach_cmd) = self.on_attach
                    && self.attach_pane_id.is_none()
                    && let Some(id) = self.find_pane_by_command(&manifest, attach_cmd)
                {
                    self.attach_pane_id = Some(id);
                }
            }
            Event::BeforeClose => {
                if self.is_attached
                    && let Some(cmd) = self.on_detach.clone()
                {
                    self.run_command_detach(&cmd);
                }
            }
            _ => {}
        }
        true
    }

    fn render(&mut self, _rows: usize, _cols: usize) {
        println!(" zj-hooker ");
    }
}

impl State {
    fn matches_pattern(&self, session_name: &str, pattern: &str) -> bool {
        if pattern.starts_with('*') && pattern.ends_with('*') {
            let middle = &pattern[1..pattern.len() - 1];
            session_name.contains(middle)
        } else if let Some(prefix) = pattern.strip_suffix('*') {
            session_name.starts_with(prefix)
        } else if let Some(suffix) = pattern.strip_prefix('*') {
            session_name.ends_with(suffix)
        } else {
            session_name == pattern
        }
    }

    fn resolve_command(&self, cmd: &str) -> String {
        if let Some(path) = cmd.strip_prefix("file:") {
            format!("bash {}", path)
        } else {
            cmd.to_string()
        }
    }

    fn run_command_attach(&mut self, cmd: &str) {
        let cmd = self.resolve_command(cmd);

        let mode = if cmd.starts_with("bash ") && cmd.contains(".sh") || cmd.contains("sudo") {
            AttachMode::Interactive
        } else {
            self.attach_mode
        };
        eprintln!("[zj-hooker] run cmd on_attach: {cmd} mode={:?}", mode);

        if let Some(pane_id) = self.attach_pane_id {
            eprintln!("[zj-hooker] rerunning pane {}", pane_id);
            rerun_command_pane(pane_id);
            return;
        }

        if let Some(ref manifest) = self.pane_manifest
            && let Some(existing_id) = self.find_pane_by_command(manifest, &cmd)
        {
            eprintln!("[zj-hooker] reusing existing pane {}", existing_id);
            self.attach_pane_id = Some(existing_id);
            rerun_command_pane(existing_id);
            return;
        }

        if mode == AttachMode::Background {
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            if !parts.is_empty() {
                run_command(&parts, BTreeMap::new());
            }
            return;
        }

        eprintln!("[zj-hooker] using interactive pane");
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }
        let (command, args) = parts.split_at(1);
        let cmd_struct = CommandToRun {
            path: PathBuf::from(command[0]),
            args: args.iter().map(|s| s.to_string()).collect(),
            cwd: None,
        };
        open_command_pane_floating(cmd_struct, None, BTreeMap::new());
    }

    fn run_command_detach(&self, cmd: &str) {
        let cmd = self.resolve_command(cmd);
        eprintln!("[zj-hooker] run cmd on_detach: {cmd}");
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }
        run_command(&parts, BTreeMap::new());
    }

    fn find_pane_by_command(&self, manifest: &PaneManifest, cmd: &str) -> Option<u32> {
        for panes in manifest.panes.values() {
            for pane in panes {
                if let Some(ref terminal_command) = pane.terminal_command
                    && terminal_command.contains(cmd)
                {
                    return Some(pane.id);
                }
            }
        }
        None
    }
}
