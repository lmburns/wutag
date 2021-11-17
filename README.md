# wutag 🔱🏷️
[![master](https://github.com/vv9k/wutag/actions/workflows/master.yml/badge.svg)](https://github.com/vv9k/wutag/actions/workflows/master.yml)
A command line tool for colorfully tagging files

NOTE: This program uses the nightly compiler for the feature `adt_const_params`, which allows using `const` parameters in functions.

### Todo
* [ ] Cleanup `README`
* [ ] Prevent the need of nightly compiler
* [ ] Fix `any` vs the normal `all` with search (it doesn't work)
* [ ] Add global option to `cp`
* [ ] Add something to remove tag if file is encountered and has a tag that is being set but is not in current registry
* [ ] Allow `-e ext` without glob pattern
* [ ] Add more tests
* [ ] Guarantee that registry changes work with tests
* [ ] Add usage examples and images

## Flags
These flags apply to mostly all all commands. If the command involves a pattern, then all flags will apply.
Also, see `--help` for the main binary or any subcommand for longer explanations of most options.
```sh
OPTIONS:
    -v, --verbose              Display debugging messages on 4 levels (i.e., -vv..)
    -d, --dir <dir>            Specify starting path for filesystem traversal
    -m, --max-depth <num>      Increase maximum recursion depth from 2
    -R, --registry <reg>       Specify a different registry to use
    -i, --case_insensitive     Case insensitively search
    -s, --case_sensitive       Case sensitively search
    -r, --regex                Search with a regular expressions
    -g, --global               Apply operation to all tags and files instead of locally
    -l, --ls-colors            Respect 'LS_COLORS' environment variable when coloring the output
    -c, --color <when>         When to colorize output
    -t, --type <filetype>      File-type(s) to filter by: f|file, d|directory, l|symlink, e|empty
    -e, --ext <extension>      Filter results by file extension
    -E, --exclude <pattern>    Exclude results that match pattern
    -h, --help                 Print help information
    -V, --version              Print version information
```

## Subcommands
`list`                 Lists all available tags or files
`set`                  Set tag(s) on files that match the given pattern
`rm`                   Remove tag(s) from the files that match the provided pattern
`clear`                Clears all tags of the files that match the provided pattern
`search`               Searches for files that have all of the provided 'tags'
`cp`                   Copies tags from the specified file to files that match a pattern
`view`                 View the results in an editor (optional pattern)
`edit`                 Edits a tag's color
`clean-cache`          Clean the cached tag registry
`print-completions`    Prints completions for the specified shell to directory or stdout

---
### `list`
```sh
FLAGS:
    -r, --raw        If provided output will be raw so that it can be easily piped to other commands
    -h, --help       Print help information
    -v, --verbose    Display debugging messages on 4 levels (i.e., -vv..)

SUBCOMMANDS:
    tags
    files
```

#### `list files`
Note: `list tags` only has notable option, which is `--border`.
```sh
FLAGS:
    -h, --help         Print help information
    -t, --with-tags    Display tags along with the files
    -f, --format       Format the tags and files output into columns
    -b, --border       Use border separators when formatting output
    -v, --verbose      Display debugging messages on 4 levels (i.e., -vv..)
    -G, --garrulous    Display tags and files on separate lines
```

#### Examples
```sh
wutag -g list files -t   # List all files with tags
wutag list files -tfb    # List files in cwd with formatted tags + borders
wutag list files -tfb    # List files in cwd with formatted tags + borders
wutag -g list tags -b    # List all tags with borders
```

---
### `set`
```sh
USAGE:
    wutag [FLAG/OPTIONS] set [FLAG/OPTIONS] <pattern> <tag>

ARGS:
    <PATTERN>    A glob pattern like "*.png"
    <TAGS>...

FLAGS:
    -q, --quiet      Do not show errors that tag already exists
    -c, --clear      Clear all tags before setting them
    -h, --help       Print help information
    -v, --verbose    Display debugging messages on 4 levels (i.e., -vv..)

OPTIONS:
    -C, --color <COLOR>    Explicitly select color for tag
```

#### Examples
```sh
wutag -E src/ -e rs -e go set '*' <tag>       # Exclude src/ & set all files with 'rs' or 'go' extension to <tag>
wutag -E src/ set '*{rs,go}' <tag>            # Tag all 'rs' and 'go' files
wutag -E src/ -r set '.*\.(rs|go)' <tag>      # Same as above except as a regular expression
wutag -i set '*glob' <tag> --color="#EF1D55"  # Ignore case and set specific color
wutag -d ~/dir set '*glob' <tag>              # Set tag in another directory
wutag -R ~/dir/new.reg -td set '*glob' <tag>  # Set tag in another registry on directories
wutag set --clear '*glob' <tag>               # Clear the tags before setting the new ones
```

---
### `rm`
Has no special options. All main binary options apply.

---
### `clear`
Clears all tags from files matching globs. This can also be used to clear tags from files that are still in the registry but are no longer on the file-system, but using the command `wutag clear --non-existent`


## Differences with my fork and the original
#### New directory locations
* [x] `macOS` now uses the following locations:
    * [x] `$HOME/.cache` instead of `$HOME/Library/Caches` for `wutag.registry`
    * [x] `$HOME/.config` instead of `$HOME/Library/Application Support` for `wutag.yml`
    * The reason for this is because I do not like spaces in my filenames
    * and I use the `XDG` specifications when using `macOS`

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

#### Searching
* [x] Case insensitive globbing applies to any pattern, as well as the `-g|--global` option
* [x] Can asynchronously use a regular expression instead of a glob with `-r|--regex`
* [x] Can search using file extensions using `-e|--ext`
    * Works both globally and locally
    * This is instead of the glob `*.{rs,md}` or the regex `.*.(rs|md)`
        * Must use `*` pattern at least for now
    * Global example: `wutag -ge 'rs' -e 'md' rm '*' txt` (only with `rm`, `clear`)
    * Local example: `wutag -e 'rs' rm '*' txt`
    * Code was modified from [`fd`](https://github.com/sharkdp/fd)
* [x] Can exclude files with the `-E|--exclude` option (works on any subcommand requiring a path)
    * Works both globally and locally
    * Global example: `wutag -gE '*exclude_path*' rm '*.txt' txt` (only with `rm`, `clear`)
    * Local example: `wutag -E 'path/to/exclude/' rm '*.txt' txt`
* [x] Can ignore certain paths permanently by using `ignores` in your configuration (example below)
* [x] Default is to now search by a pattern and an optional tag
* [x] Searching is also now local by default
    * `wutag -g search <pattern> <optional_tag>`
    * To search just by using a tag, use `*` as a pattern
* [x] Can filter results by file type using `-t|--type` with any subcommand requiring a pattern

#### Multiple registries
* [x] Multiple registries are available with the `-R|--registry` option
    * (Maybe) Add registry to `ERROR` message (would be difficult to implement, have to use registry in the metadata)
    * Registries can also be used through the `WUTAG_REGISTRY` environment variable
    * Tildes (`~`), and other environment variables can be used when declaring the registry:
```sh
`WUTAG_REGISTRY="$XDG_CONFIG_HOME/wutag/my.registry wutag set '*.rs' rust"`
```

#### Deleted files
* Used to only show an error if `clear`ing a file that doesn't exist. Now, it won't
* To remove files/directories from the registry which no longer exist, use the `-n|--non-existent` flag (must be used with `-g|--global`)
    * `wutag --global clear --non-existent '*'`

#### Default command
* [x] Use `wutag list files -t` as a default command if there are none listed (i.e., using only `wutag`)
    * Trying to decide whether or not local or global should be default

#### Aliases and subcommand inferencing
* [x] Alias `list` with `ls` and infer all other subcommands, i.e., `clean` == `clean-cache`; `p`, `pr`, `pri` ... == `print-completions`
    * As long as the command can be clearly inferred with no ambiguity

#### New command similar to what `add` vs `set` would be
* [x] Differentiate between `set` and `add` (added `wutag set --clear`)
    * May instead use `add` and `set` at some point
    * For the time being, `set --clear` will `clear` the tags before setting them

#### Completions
* [x] List tags and use them for completions
    * Improves completion capabilities
    * When using any command that requires an existing tag, pressing `<tab>` will autocomplete
    * `clap::ValueHints` is also used to complete paths and files

#### Color
* [x] Option to force colored output on pipe with `--color=(always|auto|never)`
* [x] `wutag` respects the `NO_COLOR` environment variable when displaying output (that is `export NO_COLOR=1`)
* [x] `-l|--ls-colors` will colorize files only with the colors specified in `LS_COLORS|LSCOLORS`
* [x] `set` allows user to override configuration by specifying a color with `-C/--color`
* [x] Configure the base file path color (example below)

#### File execution
* [x] Can execute external commands on matching files
    * Normal `fd` placeholders can be used
    * A new placeholder `{..}` will execute `wutag` commands on the file
    * For example: `wutag -g search markdown -x {..} set {/} new_tag`
    * If file path is `/home/user/testing/home/main.rs`
        * `{..}` expands to `wutag -d /home/user/testing/home`
        * `{/}` expands to `main.rs`
        * **[TIP]**: Use `... -x {@} ...` for forced colored output
        * Other tokens:
            * `{@s}` sets a tag (e.g., `wutag search '*.rs' -x {@s} new`)
            * `{@r}` removes a tag
            * `{@x}` clears tags (no other argument is required)
            * `{@c}` copies tags to a pattern

```sh
# {@c}
wutag -g search '*.txt' -t xx -x {@c} '*.toml'
```

#### Edit tags in `$EDITOR`
```sh
wutag view --all -p <pattern> # view *all* files matching pattern
wutag view                    # view all files that are already tagged
wutag view -a -f json         # view all files that are already tagged in json format
```

#### Set tags through `stdin`
* Example:

```sh
fd -e rs '*main*' | wutag set --stdin tag1 tag2
# Note that --stdin does not need to be explicitly called
fd -e rs '*main*' | wutag set tag1 tag2
```

![Example usage](https://github.com/vv9k/wutag/blob/master/static/usage.svg)

## Install

If you use arch Linux and have AUR repositories set up you can use your favorite AUR manager to download `wutag`. For example with `paru`:
 - `paru -S wutag`
 - or latest master branch with `paru -S wutag-git`

If you're on another Linux distribution or macOS you can download one of the pre-built binaries from [here](https://github.com/vv9k/wutag/releases).

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
base_color: "#FF5813"       # default color of file path
border_color: "#A06469"     # default color when using `list files -tfb`
format: "yaml"              # default command when viewing tags in editor
max_depth: 100              # maximum depth to recurse when applying function to files
colors:                     # list of colors to choose from when setting tags
    - '0xabba0f'            # can be in formats other than #RRGGBB
    - '#121212'
    - '0x111111'
ignores:                    # list of paths to always ignore
    - "src/"
    - "Library/"
    - "**/foo/bar"
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
    wutag [FLAGS/OPTIONS] <SUBCOMMAND> [TAGS/FLAGS]

FLAGS:
    -h, --help                Print help information
    -V, --version             Print version information
    -i, --case-insensitive    Case insensitively search
    -r, --regex               Case insensitively search
    -g, --global              Apply operation to all tags and files instead of locally
    -l, --ls-colors           Respect 'LS_COLORS' environment variable when coloring the output
    -v, --verbose             Display debugging messages on 4 levels (i.e., -vv..)

OPTIONS:
    -d, --dir <DIR>...            Specify starting path for filesystem traversal
    -m, --max-depth <num>         Increase maximum recursion depth from 2
    -R, --registry <REG>          Specify a different registry to use
    -c, --color <when>            When to colorize output
    -e, --ext <extension>...      Filter results by file extension
    -E, --exclude <pattern>...    Exclude results that match pattern

SUBCOMMANDS:
    list                 Lists all available tags or files
    set                  Set tag(s) on files that match the given pattern
    rm                   Remove tag(s) from the files that match the provided pattern
    clear                Clears all tags of the files that match the provided pattern
    search               Searches for files that have all of the provided 'tags'
    cp                   Copies tags from the specified file to files that match a pattern
    edit                 Edits a tag
    clean-cache          Clean the cached tag registry
    print-completions    Prints completions for the specified shell to stdout

See wutag --help for longer explanations of some base options.
Use --help after a subcommand for explanations of more options.
```

### More help
Use the `--help` flag for longer explanations on some flags, as well as `--help|-h` after each subcommand
to see the available options. Tip: If completions are installed it will help a ton.

### Credit
* This is a fork. Original can be found [here](https://github.com/vv9k/wutag)
* Also want to thank [sharkdp's fd](https://github.com/sharkdp/fd) repository, because some of the code and ideas came from there


## License
[MIT](https://github.com/vv9k/wutag/blob/master/LICENSE)
