use std::path::Path;

use crate::config::IconsMode;
use crate::model::LivePane;
use crate::rows::ProgressState;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaneVisualState {
    Focused,
    Shell,
    Command,
    Codex,
    Claude,
    Editor,
    Floating,
    Fullscreen,
    Stacked,
    Exited,
    Unknown,
}

pub fn classify_pane(
    pane: &LivePane,
    command: Option<&[String]>,
    is_foreground: Option<bool>,
    current_session: bool,
) -> PaneVisualState {
    if pane.exited {
        return PaneVisualState::Exited;
    }
    if pane.is_focused {
        return PaneVisualState::Focused;
    }
    if let Some(command) = command {
        return classify_command_with_foreground(command, is_foreground);
    }
    if current_session {
        if let Some(initial_command) = pane.initial_command.as_deref() {
            let command = initial_command
                .split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>();
            let state = classify_command(&command);
            if state != PaneVisualState::Unknown {
                return state;
            }
        }
    }
    if pane.is_suppressed {
        PaneVisualState::Stacked
    } else if pane.is_fullscreen {
        PaneVisualState::Fullscreen
    } else if pane.is_floating {
        PaneVisualState::Floating
    } else {
        PaneVisualState::Unknown
    }
}

pub fn classify_command(command: &[String]) -> PaneVisualState {
    let Some(executable) = command.first() else {
        return PaneVisualState::Shell;
    };
    let executable = Path::new(executable)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(executable)
        .trim_end_matches(".exe")
        .to_ascii_lowercase();

    if executable.starts_with("codex") {
        PaneVisualState::Codex
    } else if executable.starts_with("claude") {
        PaneVisualState::Claude
    } else if matches!(
        executable.as_str(),
        "nvim" | "vim" | "vi" | "hx" | "helix" | "nano" | "emacs"
    ) {
        PaneVisualState::Editor
    } else if matches!(
        executable.as_str(),
        "sh" | "bash" | "zsh" | "fish" | "nu" | "nushell" | "xonsh" | "pwsh"
    ) {
        PaneVisualState::Shell
    } else {
        PaneVisualState::Command
    }
}

pub fn is_running_command(command: &[String]) -> bool {
    !matches!(
        classify_command(command),
        PaneVisualState::Shell | PaneVisualState::Unknown
    )
}

pub fn command_is_running(command: &[String], is_foreground: Option<bool>) -> bool {
    match is_foreground {
        Some(is_foreground) => is_foreground,
        None => is_running_command(command),
    }
}

pub fn progress_state(
    pane: &LivePane,
    command: Option<&[String]>,
    is_foreground: Option<bool>,
) -> ProgressState {
    if pane.exited {
        return ProgressState::Exited;
    }
    let Some(command) = command else {
        return ProgressState::Unknown;
    };
    match is_foreground {
        Some(true) => ProgressState::Running,
        Some(false) => ProgressState::Idle,
        None => match classify_command(command) {
            PaneVisualState::Shell => ProgressState::Idle,
            PaneVisualState::Unknown => ProgressState::Unknown,
            _ => ProgressState::Running,
        },
    }
}

fn classify_command_with_foreground(
    command: &[String],
    is_foreground: Option<bool>,
) -> PaneVisualState {
    match (classify_command(command), is_foreground) {
        (_, Some(false)) => PaneVisualState::Shell,
        (PaneVisualState::Shell, Some(true)) => PaneVisualState::Command,
        (state, _) => state,
    }
}

pub fn icon_for(state: PaneVisualState, mode: IconsMode) -> &'static str {
    match mode {
        IconsMode::Nerd => match state {
            PaneVisualState::Focused => "●",
            PaneVisualState::Shell => "$",
            PaneVisualState::Command => "›",
            PaneVisualState::Codex => "❉",
            PaneVisualState::Claude => "✳",
            PaneVisualState::Editor => "󰈙",
            PaneVisualState::Floating => "󰹙",
            PaneVisualState::Fullscreen => "󰊓",
            PaneVisualState::Stacked => "󰐕",
            PaneVisualState::Exited => "✕",
            PaneVisualState::Unknown => "?",
        },
        IconsMode::Plain => match state {
            PaneVisualState::Focused => "*",
            PaneVisualState::Shell => "$",
            PaneVisualState::Command => ">",
            PaneVisualState::Codex => "C",
            PaneVisualState::Claude => "A",
            PaneVisualState::Editor => "E",
            PaneVisualState::Floating => "F",
            PaneVisualState::Fullscreen => "M",
            PaneVisualState::Stacked => "S",
            PaneVisualState::Exited => "X",
            PaneVisualState::Unknown => "?",
        },
    }
}
