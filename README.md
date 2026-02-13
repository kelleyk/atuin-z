# atuin-z

Frecency-based directory jumping powered by your [Atuin](https://github.com/atuinsh/atuin) shell history.

atuin-z works like [z](https://github.com/rupa/z) or [zoxide](https://github.com/ajeetdsouza/zoxide), but instead of maintaining its own tracking database, it reads your existing Atuin history database (read-only). Every command Atuin records includes the working directory it was run from — atuin-z uses this data to rank directories by frecency (frequency weighted by recency).

## Installation

Build from source (requires Rust):

```sh
cargo install --path .
```

Then add the shell integration to your shell config:

**Zsh** (`~/.zshrc`):
```sh
eval "$(atuin-z init zsh)"
```

**Bash** (`~/.bashrc`):
```sh
eval "$(atuin-z init bash)"
```

**Fish** (`~/.config/fish/config.fish`):
```fish
atuin-z init fish | source
```

## Usage

```sh
z foo          # cd to the highest-ranked directory matching "foo"
z foo bar      # cd to the highest-ranked directory matching both "foo" and "bar"
z              # cd ~

z -l           # list all directories with scores
z -l foo       # list all directories matching "foo" with scores
z -c foo       # restrict matches to subdirectories of the current directory

z -r foo       # rank by frequency only (ignore recency)
z -t foo       # rank by recency only (ignore frequency)

z -x           # exclude the current directory from results
z -x /some/dir # exclude a specific directory from results
```

## How it works

### Scoring

By default, atuin-z scores directories using frecency — frequency weighted by a recency bucket:

| Most recent visit | Weight |
|---|---|
| Within the last hour | frequency x 4 |
| Within the last day | frequency x 2 |
| Within the last week | frequency x 0.5 |
| Older | frequency x 0.25 |

Use `-r` for frequency-only ranking or `-t` for recency-only ranking.

### Matching

All keywords must match as case-insensitive substrings of the directory path (AND logic). If the last keyword matches the final path component (the basename), the result gets a score boost. Directories that no longer exist on disk are filtered out automatically.

### Database resolution

atuin-z locates the Atuin history database using the same priority chain as Atuin itself:

1. `--db <path>` CLI flag
2. `ATUIN_DB_PATH` environment variable
3. `$ATUIN_DATA_DIR/history.db`
4. `$XDG_DATA_HOME/atuin/history.db`
5. `~/.local/share/atuin/history.db`

The database is always opened read-only.

### Exclusions

Since atuin-z doesn't own the Atuin database, the `-x` flag maintains a separate exclusion list at `~/.local/share/atuin-z/exclusions` (or `$XDG_DATA_HOME/atuin-z/exclusions`). Excluded directories are filtered from all results.

## License

MIT
