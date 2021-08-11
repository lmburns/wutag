# wutag 🔱🏷️
[![master](https://github.com/vv9k/wutag/actions/workflows/master.yml/badge.svg)](https://github.com/vv9k/wutag/actions/workflows/master.yml)
A command line tool for tagging files

## Fork
#### New directory locations
* [x] `macOS` now uses the following locations:
    * [x] `$HOME/.cache` instead of `$HOME/Library/Caches` for `wutag.registry`
    * [x] `$HOME/.config` instead of `$HOME/Library/Application Support` for `wutag.yml`

#### Global option
* [x] `list`, `rm`, `clear`, and `search` have `--global` option to match only on files that are already tagged
    * `wutag -g rm '**/z*.md' flag_name`
* [x] `list [FLAGS] (files|tags) [OPTS]` is local by default. Use `-g|--global` to view all tagged files

#### Display and formatting
* [x] `list files -t` does not display full path of files unless `-g|--global` is used. Instead it is directory-relative
* [x] `list files -tG` displays `tags` and `files` on separate lines (`--garrulous` is taken from [`tag`](https://github.com/jdbery/tag))
* [x] `list tags` displays the count of each tag
* [x] `list files -tf` displays `tags` and `files` in a column `-f`ormat (requires `-t|--with-tags`)
* [x] Display a success message of which registry is cleaned when clearing cache with `clean-cache`

#### Improved globbing
* [x] Case insensitive globbing applies to any pattern, as well as the `-g|--global` option
* [x] Both `--global` and non-`global` options have extended features
    * [`globwalk`](https://github.com/Gilnaa/globwalk) for non-`global`
    * [`globset`](https://docs.rs/globset/0.4.8/globset/#syntax) for `--global`
        * This has more features than the above
        * Only applies to `rm` and `clear` commands when using `--global`
* [ ] Am looking for a crate with good extended globbing
    * [`globber`](https://docs.rs/globber/0.1.3/globber/index.html#syntax) has more features, but doesn't support case-insensitivity

#### Multiple registries
* [x] Multiple registries are available with the `-r|--registry` option
* [ ] FIXME: Error if setting a tag to name `registry` if unquoted
    * (Maybe) Add registry to `ERROR` message (would be difficult to implement, have to use registry in the metadata)
    * Registries can also be used through the `WUTAG_REGISTRY` environment variable
    * Tildes (`~`), and other environment variables can be used when declaring the registry:
```sh
`WUTAG_REGISTRY="$XDG_CONFIG_HOME/wutag/my.registry wutag set '*.rs' rust"`
```

* [x] `wutag` respects the `NO_COLOR` environment variable when displaying output (that is `export NO_COLOR=1`)

#### Default command
* [x] Use `wutag list files -t` as a default command if there are none listed (i.e., using only `wutag`)
    * Trying to decide whether or not local or global should be default

#### Aliases and subcommand inferencing
* [x] Alias `list` with `ls` and infer all other subcommands, i.e., `clean` == `clean-cache`; `p`, `pr`, `pri` ... == `print-completions`
    * As long as the command can be clearly inferred with no ambiguity

#### New command similar to what `add` vs `set` would be
* [x] Differentiate between `set` and `add` (added `wutag set --clear`)
    * May instead use `add` and `set` at some point
    * For the time being, `set --clear` will `clear` the tags before setting it

#### Completions
* [x] List tags and use them for completions
    * Improves completion capabilities
    * When using any command that requires an existing tag, pressing `<tab>` will autocomplete
    * `clap::ValueHints` is also used to complete paths and files

#### Todo
* [ ] Find way to force colored output on pipe
* [ ] Configuration option for base file color
* [ ] Fix `any` vs the normal `all` (it doesn't work)
* [ ] `raw` does not work with formatted (fix or find way to implement `conflicts_with`)

![Example usage](https://github.com/vv9k/wutag/blob/master/static/usage.svg)

## Install

If you use arch Linux and have AUR repositories set up you can use your favourite AUR manager to download `wutag`. For example with `paru`:
 - `paru -S wutag`
 - or latest master branch with `paru -S wutag-git`

If you're on another Linux distribution or MacOS you can download one of the prebuilt binaries from [here](https://github.com/vv9k/wutag/releases).

To build manually you'll need latest `rust` and `cargo`. Build with:
 - `cargo build --release`

## Usage

By default each tag will be assigned with a random color from 8 base colors (either bright or normal so 16 colors in total). You can later edit each tag by using `edit` subcommand like this:
 - `wutag edit school --color 0x1f1f1f`
 - or `wutag edit code --color '#ff00aa'`
 - or `wutag edit work --color FF0000`
 - The colors are case insensitive

Each command that takes a pattern starts a filesystem traversal from current working directory. To override this
behaviour specify a global parameter `--dir` or `-d` like this:
 - `wutag -d ~ set '**' code`

Default recursion depth is set to *2*. To increase it use `--max-depth` or `-m` global parameter.

After tagging your files with `set` like:
 - `wutag set '*.jpg' photos`
 - `wutag set 'DCIM_12*' doge`
you can easily get the list of files with specified tags by doing `wutag search photos doge`.

To utilize the list by other programs pass the `--raw` or `-r` flag to `search` subcommand like:
 - `wutag search -r --any cat doge | xargs rm -rf  # please don't do this :(`.

When `--any` flag is provided as in the example `wutag` will match files containing any of the provided tags rather than all of them.

If you are into emojis then surely you can use emojis to tag files 🙂 ```wutag set '*.doc' 📋```

## Configuration

`wutag` lets you configure base colors used when creating tags or modify other settings globally.
There will be a `wutag.yml` file located in `$XDG_CONFIG_HOME/wutag` or `$HOME/.config/wutag` with only `max_depth` in it. Colors can be added like so:

Example configuration:
```yaml
---
max_depth: 100
colors:
- '0xabba0f'
- '#121212'
- '0x111111'
```

## Tab completion

To get tab completion use `wutag print-completions --shell <shell> > /path/to/completions/dir/...` to enable it in your favorite shell.

Available shells are:
 - `bash`
 - `elvish`
 - `fish`
 - `powershell`
 - `zsh`

 To enable completions on the fly use:
 - `. <(wutag print-completions --shell zsh)`


## User interface
### Usage
```
USAGE:
    wutag [FLAGS] [OPTIONS] <SUBCOMMAND>

FLAGS:
    -i, --case-insensitive    Case insensitively search
    -g, --global              Apply operation to all tags and files instead of locally
    -h, --help                Prints help information
    -n, --no-color            Do not colorize the output [env: NO_COLOR=]
    -V, --version             Prints version information

OPTIONS:
    -d, --dir <dir>
            Specify starting path for filesystem traversal

    -m, --max-depth <max-depth>
            Increase maximum recursion depth (default: 2)

    -r, --registry <reg>
            Specify a different registry to use


SUBCOMMANDS:
    add                  Add tag(s) to files that match the given pattern
    clean-cache          Clean the cached tag registry
    clear                Clears all tags of the files that match the provided pattern
    cp                   Copies tags from the specified file to files that match a pattern
    edit                 Edits a tag
    list                 Lists all available tags or files
    rm                   Remove tag(s) from the files that match the provided pattern
    search               Searches for files that have all of the provided 'tags'
    set                  Set tag(s) on files that match the given pattern
    print-completions    Prints completions for the specified shell to stdout
```

### More help
Use the `--help` flag for longer explanations on some flags, as well as `--help|-h` after each subcommand
to see the available options. Tip: If completions are installed it will help a ton.

## License
[MIT](https://github.com/vv9k/wutag/blob/master/LICENSE)
