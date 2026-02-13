use crate::cli::Shell;

pub fn init(shell: &Shell) -> &'static str {
    match shell {
        Shell::Bash => include_str!("shell/bash.sh"),
        Shell::Zsh => include_str!("shell/zsh.sh"),
        Shell::Fish => include_str!("shell/fish.fish"),
    }
}
