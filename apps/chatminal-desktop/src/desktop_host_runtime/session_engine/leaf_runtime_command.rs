use std::path::{Path, PathBuf};

use portable_pty::CommandBuilder;

pub fn prepare_leaf_command(command: &mut CommandBuilder) -> Result<(), String> {
    command.env("TERM", "xterm-256color");
    command.env_remove("COLUMNS");
    command.env_remove("LINES");
    apply_macos_zsh_startup_shim(command)
}

fn apply_macos_zsh_startup_shim(command: &mut CommandBuilder) -> Result<(), String> {
    let Some(program) = command.get_argv().first() else {
        return Ok(());
    };
    if !cfg!(target_os = "macos") || !is_zsh_shell(program) {
        return Ok(());
    }

    let shim_dir = ensure_macos_zsh_startup_shim_dir()?;
    let original_zdotdir = std::env::var_os("ZDOTDIR")
        .filter(|value| !value.is_empty())
        .or_else(|| std::env::var_os("HOME"))
        .unwrap_or_else(|| PathBuf::from("/").into_os_string());

    command.env("CHATMINAL_ORIGINAL_ZDOTDIR", original_zdotdir);
    command.env("ZDOTDIR", shim_dir.as_os_str());
    Ok(())
}

fn is_zsh_shell(shell: &std::ffi::OsStr) -> bool {
    Path::new(shell)
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("zsh"))
}

fn ensure_macos_zsh_startup_shim_dir() -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join(format!("chatminal-zsh-startup-{}", std::process::id()));
    std::fs::create_dir_all(&dir)
        .map_err(|err| format!("create zsh startup shim dir failed: {err}"))?;

    let zshenv = r#"typeset -gx CHATMINAL_SHIM_ZDOTDIR=\"$ZDOTDIR\"
if [ -n \"${CHATMINAL_ORIGINAL_ZDOTDIR:-}\" ] && [ -r \"${CHATMINAL_ORIGINAL_ZDOTDIR}/.zshenv\" ]; then
  source \"${CHATMINAL_ORIGINAL_ZDOTDIR}/.zshenv\"
elif [ -r \"$HOME/.zshenv\" ]; then
  source \"$HOME/.zshenv\"
fi
typeset -gx ZDOTDIR=\"$CHATMINAL_SHIM_ZDOTDIR\"
"#;
    let zshrc = r#"if [ -n \"${CHATMINAL_ORIGINAL_ZDOTDIR:-}\" ] && [ -r \"${CHATMINAL_ORIGINAL_ZDOTDIR}/.zshrc\" ]; then
  source \"${CHATMINAL_ORIGINAL_ZDOTDIR}/.zshrc\"
elif [ -r \"$HOME/.zshrc\" ]; then
  source \"$HOME/.zshrc\"
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
