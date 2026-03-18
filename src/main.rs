//!
//! This is a plugin for [zellij](https://github.com/zellij-org/zellij) that allows you to run
//! commands on session attach and detach.
//!
//! # Example configuration
//! ```kdl
//!
//! plugin location="file:~/zellij-plugins/zj-hooker.wasm" {
//!     "session_pattern" "main"           # exact match
//!     "session_pattern" "dev*"            # prefix match
//!     "session_pattern" "*dev"            # suffix match
//!     "session_pattern" "*dev*"          # contains match
//!     "on_attach" "echo 'attached to matching session'"
//!     "on_detach" "echo 'detached from matching session'"
//! }
//! ```
//!
use std::collections::BTreeMap;
use zellij_tile::prelude::*;

static ON_ATTACH_KEY: &str = "on_attach";
static ON_DETACH_KEY: &str = "on_detach";
static SESSION_KEY: &str = "session_pattern";

#[derive(Default)]
struct State {
    on_attach: Option<String>,
    on_detach: Option<String>,
    session_pattern: Option<String>,
    client_count: usize,
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        request_permission(&[PermissionType::RunCommands, PermissionType::ReadApplicationState]);

        self.on_attach = configuration.get(ON_ATTACH_KEY).cloned();
        self.on_detach = configuration.get(ON_DETACH_KEY).cloned();
        self.session_pattern = configuration.get(SESSION_KEY).cloned();

        eprintln!("zj-hooker: load called, on_attach={:?}, on_detach={:?}, pattern={:?}", 
            self.on_attach, self.on_detach, self.session_pattern);

        subscribe(&[EventType::ListClients, EventType::BeforeClose, EventType::Visible, EventType::RunCommandResult]);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::ListClients(clients) => {
                let current_count = clients.len();
                eprintln!("zj-hooker: ListClients event, count: {}", current_count);

                let was_empty = self.client_count == 0;
                let client_joined = current_count > self.client_count;

                if was_empty || client_joined {
                    eprintln!(
                        "zj-hooker: Client attached (was_empty={}, joined={})",
                        was_empty, client_joined
                    );

                    let current_session = std::env::var("ZELLIJ_SESSION_NAME").unwrap_or_default();
                    eprintln!("zj-hooker: Current session: {}", current_session);

                    if !current_session.is_empty() {
                        if let Some(ref pattern) = self.session_pattern {
                            let matches = self.matches_pattern(&current_session, pattern);
                            eprintln!("zj-hooker: Pattern '{}' matches: {}", pattern, matches);
                            if matches && let Some(ref cmd) = self.on_attach {
                                eprintln!("zj-hooker: Running on_attach: {}", cmd);
                                self.run_command(cmd);
                            }
                        } else if let Some(ref cmd) = self.on_attach {
                            eprintln!("zj-hooker: Running on_attach: {}", cmd);
                            self.run_command(cmd);
                        }
                    }
                }

                self.client_count = current_count;
            }
            Event::BeforeClose => {
                eprintln!("zj-hooker: BeforeClose event");
                let current_session = std::env::var("ZELLIJ_SESSION_NAME").unwrap_or_default();

                if let Some(ref pattern) = self.session_pattern {
                    if !current_session.is_empty() {
                        let matches = self.matches_pattern(&current_session, pattern);
                        if matches && let Some(ref cmd) = self.on_detach {
                            eprintln!("zj-hooker: Running on_detach: {}", cmd);
                            self.run_command(cmd);
                        }
                    }
                } else if let Some(ref cmd) = self.on_detach {
                    eprintln!("zj-hooker: Running on_detach: {}", cmd);
                    self.run_command(cmd);
                }
            }
            _ => {}
        }
        true
    }

    fn render(&mut self, _rows: usize, _cols: usize) {
        println!(" zj-hooker ");
        if let Some(ref sessions) = self.session_pattern {
            println!(" sessions: {}", sessions);
        }
        if let Some(ref on_attach) = self.on_attach {
            println!(" on_attach: {}", on_attach);
        }
        if let Some(ref on_detach) = self.on_detach {
            println!(" on_detach: {}", on_detach);
        }
        println!();
        println!(" Waiting for attach/detach events... ");
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

    fn run_command(&self, cmd: &str) {
        eprintln!("zj-hooker: run_command called with: {}", cmd);

        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            eprintln!("zj-hooker: command is empty");
            return;
        }

        let (program, args) = parts.split_first().expect("empty command");
        eprintln!(
            "zj-hooker: opening pane with program: {}, args: {:?}",
            program, args
        );

        open_command_pane_floating(
            CommandToRun::new_with_args(program, args.to_vec()),
            None,
            BTreeMap::new(),
        );
        eprintln!("zj-hooker: open_command_pane_floating called");
    }
}
