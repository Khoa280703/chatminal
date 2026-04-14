use std::path::{Path, PathBuf};

use portable_pty::CommandBuilder;

pub fn prepare_leaf_command(command: &mut CommandBuilder) -> Result<(), String> {
    command.env("TERM", "xterm-256color");
    command.env_remove("COLUMNS");
    command.env_remove("LINES");
    apply_macos_shell_startup_shim(command)
}

fn apply_macos_shell_startup_shim(command: &mut CommandBuilder) -> Result<(), String> {
    let Some(program) = command.get_argv().first() else {
        return Ok(());
    };
    if !cfg!(target_os = "macos") {
        return Ok(());
    }

    if is_zsh_shell(program) {
        return apply_macos_zsh_startup_shim(command);
    }
    if is_bash_shell(program) {
        return apply_macos_bash_startup_shim(command);
    }
    Ok(())
}

fn apply_macos_zsh_startup_shim(command: &mut CommandBuilder) -> Result<(), String> {
    let shim_dir = ensure_macos_zsh_startup_shim_dir()?;
    let original_zdotdir = std::env::var_os("ZDOTDIR")
        .filter(|value| !value.is_empty())
        .or_else(|| std::env::var_os("HOME"))
        .unwrap_or_else(|| PathBuf::from("/").into_os_string());

    command.env("CHATMINAL_ORIGINAL_ZDOTDIR", original_zdotdir);
    command.env("ZDOTDIR", shim_dir.as_os_str());
    Ok(())
}

fn apply_macos_bash_startup_shim(command: &mut CommandBuilder) -> Result<(), String> {
    if command.get_argv().len() > 1 {
        return Ok(());
    }

    let shim_dir = ensure_macos_bash_startup_shim_dir()?;
    let original_home = std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| PathBuf::from("/").into_os_string());
    let rcfile = shim_dir.join(".bashrc");

    command.env("CHATMINAL_ORIGINAL_HOME", original_home);
    command.env("CHATMINAL_BASH_SHIM_DIR", shim_dir.as_os_str());
    command.arg("--rcfile");
    command.arg(rcfile.as_os_str());
    command.arg("-i");
    Ok(())
}

fn is_zsh_shell(shell: &std::ffi::OsStr) -> bool {
    Path::new(shell)
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("zsh"))
}

fn is_bash_shell(shell: &std::ffi::OsStr) -> bool {
    Path::new(shell)
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("bash"))
}

fn ensure_macos_zsh_startup_shim_dir() -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join(format!("chatminal-zsh-startup-{}", std::process::id()));
    std::fs::create_dir_all(&dir)
        .map_err(|err| format!("create zsh startup shim dir failed: {err}"))?;

    let zshenv = r#"typeset -gx CHATMINAL_SHIM_ZDOTDIR="$ZDOTDIR"
if [ -n "${CHATMINAL_ORIGINAL_ZDOTDIR:-}" ] && [ -r "${CHATMINAL_ORIGINAL_ZDOTDIR}/.zshenv" ]; then
  source "${CHATMINAL_ORIGINAL_ZDOTDIR}/.zshenv"
elif [ -r "$HOME/.zshenv" ]; then
  source "$HOME/.zshenv"
fi
typeset -gx ZDOTDIR="$CHATMINAL_SHIM_ZDOTDIR"
"#;
    let zshrc = r#"if [ -n "${CHATMINAL_ORIGINAL_ZDOTDIR:-}" ] && [ -r "${CHATMINAL_ORIGINAL_ZDOTDIR}/.zshrc" ]; then
  source "${CHATMINAL_ORIGINAL_ZDOTDIR}/.zshrc"
elif [ -r "$HOME/.zshrc" ]; then
  source "$HOME/.zshrc"
fi
if [ -z "${HISTFILE:-}" ] || [[ "$HISTFILE" == "$CHATMINAL_SHIM_ZDOTDIR/"* ]]; then
  typeset -gx HISTFILE="${CHATMINAL_ORIGINAL_ZDOTDIR:-$HOME}/.zsh_history"
fi
# Keep shell history available across Chatminal restarts and sibling shells.
setopt APPEND_HISTORY
setopt INC_APPEND_HISTORY
setopt SHARE_HISTORY
setopt HIST_FCNTL_LOCK
if [ -n "${HISTFILE:-}" ] && [ -r "$HISTFILE" ]; then
  builtin fc -R "$HISTFILE"
fi
unsetopt PROMPT_SP
unsetopt PROMPT_CR
"#;

    std::fs::write(dir.join(".zshenv"), zshenv)
        .map_err(|err| format!("write zsh startup shim .zshenv failed: {err}"))?;
    std::fs::write(dir.join(".zshrc"), zshrc)
        .map_err(|err| format!("write zsh startup shim .zshrc failed: {err}"))?;
    Ok(dir)
}

fn ensure_macos_bash_startup_shim_dir() -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join(format!("chatminal-bash-startup-{}", std::process::id()));
    std::fs::create_dir_all(&dir)
        .map_err(|err| format!("create bash startup shim dir failed: {err}"))?;

    let bashrc = r#"if [ -n "${CHATMINAL_ORIGINAL_HOME:-}" ] && [ -r "${CHATMINAL_ORIGINAL_HOME}/.bashrc" ]; then
  . "${CHATMINAL_ORIGINAL_HOME}/.bashrc"
elif [ -r "$HOME/.bashrc" ]; then
  . "$HOME/.bashrc"
fi
if [ -z "${HISTFILE:-}" ] || [[ "$HISTFILE" == "$CHATMINAL_BASH_SHIM_DIR/"* ]]; then
  export HISTFILE="${CHATMINAL_ORIGINAL_HOME:-$HOME}/.bash_history"
fi
# Keep shell history available across Chatminal restarts and sibling shells.
shopt -s histappend
if [ -n "${HISTFILE:-}" ] && [ -r "$HISTFILE" ]; then
  builtin history -r "$HISTFILE"
fi
chatminal_sync_bash_history() {
  builtin history -a
  builtin history -n
}
case ";${PROMPT_COMMAND:-};" in
  *";chatminal_sync_bash_history;"*) ;;
  *)
    case "${PROMPT_COMMAND:-}" in
      *[![:space:]]*) PROMPT_COMMAND="chatminal_sync_bash_history; ${PROMPT_COMMAND}" ;;
      *) PROMPT_COMMAND="chatminal_sync_bash_history" ;;
    esac
    ;;
esac
export PROMPT_COMMAND
"#;

    std::fs::write(dir.join(".bashrc"), bashrc)
        .map_err(|err| format!("write bash startup shim .bashrc failed: {err}"))?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::{
        ensure_macos_bash_startup_shim_dir, ensure_macos_zsh_startup_shim_dir, prepare_leaf_command,
    };
    use portable_pty::CommandBuilder;

    #[test]
    fn zsh_startup_shim_enables_history_sharing() {
        let dir = ensure_macos_zsh_startup_shim_dir().expect("create shim dir");
        let zshrc = std::fs::read_to_string(dir.join(".zshrc")).expect("read shim zshrc");
        assert!(zshrc.contains("typeset -gx HISTFILE="));
        assert!(zshrc.contains("setopt APPEND_HISTORY"));
        assert!(zshrc.contains("setopt INC_APPEND_HISTORY"));
        assert!(zshrc.contains("setopt SHARE_HISTORY"));
        assert!(zshrc.contains("setopt HIST_FCNTL_LOCK"));
        assert!(zshrc.contains("builtin fc -R \"$HISTFILE\""));
    }

    #[test]
    fn bash_startup_shim_enables_history_sharing() {
        let dir = ensure_macos_bash_startup_shim_dir().expect("create shim dir");
        let bashrc = std::fs::read_to_string(dir.join(".bashrc")).expect("read shim bashrc");
        assert!(bashrc.contains("export HISTFILE="));
        assert!(bashrc.contains("shopt -s histappend"));
        assert!(bashrc.contains("builtin history -r \"$HISTFILE\""));
        assert!(bashrc.contains("builtin history -a"));
        assert!(bashrc.contains("builtin history -n"));
        assert!(bashrc.contains("PROMPT_COMMAND=\"chatminal_sync_bash_history"));
    }

    #[test]
    fn bash_command_uses_rcfile_shim() {
        let mut command = CommandBuilder::new("/bin/bash");
        prepare_leaf_command(&mut command).expect("prepare command");

        let argv: Vec<String> = command
            .get_argv()
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect();
        assert_eq!(argv.first().map(String::as_str), Some("/bin/bash"));
        if cfg!(target_os = "macos") {
            assert!(argv.windows(2).any(|window| {
                window[0] == "--rcfile" && window[1].contains("chatminal-bash-startup-")
            }));
            assert!(argv.iter().any(|value| value == "-i"));
        } else {
            assert_eq!(argv, vec!["/bin/bash".to_string()]);
        }
    }
}
