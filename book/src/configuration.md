# Configuration

Currently, **laze** has no configuration itself.
Only its `<TAB>` (shell-) completions need some set-up.

## Shell completions

**laze** provides dynamic shell completions (`TAB` completions in the shell).
In order to use them, they need to be set up.

**If you've installed laze using your package manager, chances are that this is already working out of the box.**

Use the snippet that corresponds to your shell to automatically load the correct
completions on shell start (if in doubt, you're probably running `bash`):

Bash
```bash
echo "source <(COMPLETE=bash laze)" >> ~/.bashrc
```

Elvish
```elvish
echo "eval (E:COMPLETE=elvish laze | slurp)" >> ~/.elvish/rc.elv
```

Fish
```fish
echo "source (COMPLETE=fish laze | psub)" >> ~/.config/fish/config.fish
```

Powershell
```powershell
echo "COMPLETE=powershell laze | Invoke-Expression" >> $PROFILE
```

Zsh
```zsh
echo "source <(COMPLETE=zsh laze)" >> ~/.zshrc
```

The shell will need to be restarted for this to take effect.
In order to just set up completions for the currently running shell session,
issue just the `source ...` command, e.g., `source <(COMPLETE=bash laze)`.
