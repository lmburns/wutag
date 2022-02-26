
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'wutag' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'wutag'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'wutag' {
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'Specify starting path for filesystem traversal')
            [CompletionResult]::new('--dir', 'dir', [CompletionResultType]::ParameterName, 'Specify starting path for filesystem traversal')
            [CompletionResult]::new('-m', 'm', [CompletionResultType]::ParameterName, 'Set maximum depth to recurse into')
            [CompletionResult]::new('--max-depth', 'max-depth', [CompletionResultType]::ParameterName, 'Set maximum depth to recurse into')
            [CompletionResult]::new('-R', 'R', [CompletionResultType]::ParameterName, 'Specify a different registry to use')
            [CompletionResult]::new('--registry', 'registry', [CompletionResultType]::ParameterName, 'Specify a different registry to use')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'When to colorize output')
            [CompletionResult]::new('--color', 'color', [CompletionResultType]::ParameterName, 'When to colorize output')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'File-type(s) to filter by: f|file, d|directory, l|symlink, e|empty')
            [CompletionResult]::new('--type', 'type', [CompletionResultType]::ParameterName, 'File-type(s) to filter by: f|file, d|directory, l|symlink, e|empty')
            [CompletionResult]::new('-e', 'e', [CompletionResultType]::ParameterName, 'Filter results by file extension')
            [CompletionResult]::new('--ext', 'ext', [CompletionResultType]::ParameterName, 'Filter results by file extension')
            [CompletionResult]::new('-E', 'E', [CompletionResultType]::ParameterName, 'Exclude results that match pattern')
            [CompletionResult]::new('--exclude', 'exclude', [CompletionResultType]::ParameterName, 'Exclude results that match pattern')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-V', 'V', [CompletionResultType]::ParameterName, 'Print version information')
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Print version information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            [CompletionResult]::new('-i', 'i', [CompletionResultType]::ParameterName, 'Case insensitively search')
            [CompletionResult]::new('--case-insensitive', 'case-insensitive', [CompletionResultType]::ParameterName, 'Case insensitively search')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'Case sensitively search')
            [CompletionResult]::new('--case-sensitive', 'case-sensitive', [CompletionResultType]::ParameterName, 'Case sensitively search')
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'Search with a regular expressions')
            [CompletionResult]::new('--regex', 'regex', [CompletionResultType]::ParameterName, 'Search with a regular expressions')
            [CompletionResult]::new('-G', 'G', [CompletionResultType]::ParameterName, 'Search with a glob pattern')
            [CompletionResult]::new('--glob', 'glob', [CompletionResultType]::ParameterName, 'Search with a glob pattern')
            [CompletionResult]::new('-F', 'F', [CompletionResultType]::ParameterName, 'Search with a literal fixed-string')
            [CompletionResult]::new('--fixed-string', 'fixed-string', [CompletionResultType]::ParameterName, 'Search with a literal fixed-string')
            [CompletionResult]::new('-g', 'g', [CompletionResultType]::ParameterName, 'Apply operation to all tags and files instead of locally')
            [CompletionResult]::new('--global', 'global', [CompletionResultType]::ParameterName, 'Apply operation to all tags and files instead of locally')
            [CompletionResult]::new('-L', 'L', [CompletionResultType]::ParameterName, 'Follow symlinks when peforming an action on a file')
            [CompletionResult]::new('--follow', 'follow', [CompletionResultType]::ParameterName, 'Follow symlinks when peforming an action on a file')
            [CompletionResult]::new('--no-follow', 'no-follow', [CompletionResultType]::ParameterName, 'Do not follow symlinks when peforming an action on a file')
            [CompletionResult]::new('-l', 'l', [CompletionResultType]::ParameterName, 'Respect ''LS_COLORS'' environment variable when coloring the output')
            [CompletionResult]::new('--ls-colors', 'ls-colors', [CompletionResultType]::ParameterName, 'Respect ''LS_COLORS'' environment variable when coloring the output')
            [CompletionResult]::new('-q', 'q', [CompletionResultType]::ParameterName, 'Do not display any output for any command')
            [CompletionResult]::new('--quiet', 'quiet', [CompletionResultType]::ParameterName, 'Do not display any output for any command')
            [CompletionResult]::new('testing', 'testing', [CompletionResultType]::ParameterValue, 'Testing new subcommands')
            [CompletionResult]::new('init', 'init', [CompletionResultType]::ParameterValue, 'Initialize the database')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'Lists all available tags or files')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Set tag(s) on files that match the given pattern')
            [CompletionResult]::new('set2', 'set2', [CompletionResultType]::ParameterValue, 'Set tag(s) on files that match the given pattern')
            [CompletionResult]::new('rm', 'rm', [CompletionResultType]::ParameterValue, 'Remove tag(s) from the files that match the provided pattern')
            [CompletionResult]::new('clear', 'clear', [CompletionResultType]::ParameterValue, 'Clears all tags of the files that match the provided pattern')
            [CompletionResult]::new('search', 'search', [CompletionResultType]::ParameterValue, 'Searches for files that have all of the provided ''tags''')
            [CompletionResult]::new('cp', 'cp', [CompletionResultType]::ParameterValue, 'Copies tags from the specified file to files that match a pattern')
            [CompletionResult]::new('view', 'view', [CompletionResultType]::ParameterValue, 'View the results in an editor (optional pattern)')
            [CompletionResult]::new('edit', 'edit', [CompletionResultType]::ParameterValue, 'Edits a tag''s color')
            [CompletionResult]::new('info', 'info', [CompletionResultType]::ParameterValue, 'Display information about the wutag environment')
            [CompletionResult]::new('repair', 'repair', [CompletionResultType]::ParameterValue, 'Repair broken/missing/modified files in the registry')
            [CompletionResult]::new('print-completions', 'print-completions', [CompletionResultType]::ParameterValue, 'Prints completions for the specified shell to dir or stdout')
            [CompletionResult]::new('clean-cache', 'clean-cache', [CompletionResultType]::ParameterValue, 'Clean the cached tag registry')
            [CompletionResult]::new('ui', 'ui', [CompletionResultType]::ParameterValue, 'Open a TUI to manage tags')
            break
        }
        'wutag;testing' {
            [CompletionResult]::new('-q', 'q', [CompletionResultType]::ParameterName, 'q')
            [CompletionResult]::new('--query', 'query', [CompletionResultType]::ParameterName, 'query')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            break
        }
        'wutag;init' {
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            break
        }
        'wutag;list' {
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'Output will not be colorized')
            [CompletionResult]::new('--raw', 'raw', [CompletionResultType]::ParameterName, 'Output will not be colorized')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            [CompletionResult]::new('tags', 'tags', [CompletionResultType]::ParameterValue, 'List the `Tags` within the database')
            [CompletionResult]::new('files', 'files', [CompletionResultType]::ParameterValue, 'List the `Files` within the database')
            break
        }
        'wutag;list;tags' {
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Print version information')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'Do not display tag count')
            [CompletionResult]::new('--no-count', 'no-count', [CompletionResultType]::ParameterName, 'Do not display tag count')
            [CompletionResult]::new('-u', 'u', [CompletionResultType]::ParameterName, 'Only display unique occurences. (See --help)')
            [CompletionResult]::new('--unique', 'unique', [CompletionResultType]::ParameterName, 'Only display unique occurences. (See --help)')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'Sort the output')
            [CompletionResult]::new('--sort', 'sort', [CompletionResultType]::ParameterName, 'Sort the output')
            [CompletionResult]::new('-i', 'i', [CompletionResultType]::ParameterName, 'Do not show implied tags')
            [CompletionResult]::new('--implied', 'implied', [CompletionResultType]::ParameterName, 'Do not show implied tags')
            [CompletionResult]::new('-1', '1', [CompletionResultType]::ParameterName, 'Display one tag per line instead of tags on files')
            [CompletionResult]::new('--one-per-line', 'one-per-line', [CompletionResultType]::ParameterName, 'Display one tag per line instead of tags on files')
            [CompletionResult]::new('-b', 'b', [CompletionResultType]::ParameterName, 'Use border separators when formatting output')
            [CompletionResult]::new('--border', 'border', [CompletionResultType]::ParameterName, 'Use border separators when formatting output')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            break
        }
        'wutag;list;files' {
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Print version information')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'Display tags along with the files')
            [CompletionResult]::new('--with-tags', 'with-tags', [CompletionResultType]::ParameterName, 'Display tags along with the files')
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'Format the tags and files output into columns')
            [CompletionResult]::new('--format', 'format', [CompletionResultType]::ParameterName, 'Format the tags and files output into columns')
            [CompletionResult]::new('-b', 'b', [CompletionResultType]::ParameterName, 'Use border separators when formatting output')
            [CompletionResult]::new('--border', 'border', [CompletionResultType]::ParameterName, 'Use border separators when formatting output')
            [CompletionResult]::new('-G', 'G', [CompletionResultType]::ParameterName, 'Display tags and files on separate lines')
            [CompletionResult]::new('--garrulous', 'garrulous', [CompletionResultType]::ParameterName, 'Display tags and files on separate lines')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            break
        }
        'wutag;set' {
            [CompletionResult]::new('-C', 'C', [CompletionResultType]::ParameterName, 'Explicitly select color for tag')
            [CompletionResult]::new('--color', 'color', [CompletionResultType]::ParameterName, 'Explicitly select color for tag')
            [CompletionResult]::new('-q', 'q', [CompletionResultType]::ParameterName, 'Do not show errors that tag already exists')
            [CompletionResult]::new('--quiet', 'quiet', [CompletionResultType]::ParameterName, 'Do not show errors that tag already exists')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'Clear all tags before setting them')
            [CompletionResult]::new('--clear', 'clear', [CompletionResultType]::ParameterName, 'Clear all tags before setting them')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 's')
            [CompletionResult]::new('--stdin', 'stdin', [CompletionResultType]::ParameterName, 'stdin')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            break
        }
        'wutag;set2' {
            [CompletionResult]::new('-C', 'C', [CompletionResultType]::ParameterName, 'Explicitly select color for tag')
            [CompletionResult]::new('--color', 'color', [CompletionResultType]::ParameterName, 'Explicitly select color for tag')
            [CompletionResult]::new('-p', 'p', [CompletionResultType]::ParameterName, 'Specify any number of `tag`=`value` pairs')
            [CompletionResult]::new('--pairs', 'pairs', [CompletionResultType]::ParameterName, 'Specify any number of `tag`=`value` pairs')
            [CompletionResult]::new('-V', 'V', [CompletionResultType]::ParameterName, 'Specify a value to set all the tag(s) to')
            [CompletionResult]::new('--value', 'value', [CompletionResultType]::ParameterName, 'Specify a value to set all the tag(s) to')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'Clear the tags on the match(es) before the new one(s) are set')
            [CompletionResult]::new('--clear', 'clear', [CompletionResultType]::ParameterName, 'Clear the tags on the match(es) before the new one(s) are set')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'Arguments are expected to be passed through stdin')
            [CompletionResult]::new('--stdin', 'stdin', [CompletionResultType]::ParameterName, 'Arguments are expected to be passed through stdin')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            break
        }
        'wutag;rm' {
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            break
        }
        'wutag;clear' {
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            break
        }
        'wutag;search' {
            [CompletionResult]::new('-x', 'x', [CompletionResultType]::ParameterName, 'Execute a command on each individual file')
            [CompletionResult]::new('--exec', 'exec', [CompletionResultType]::ParameterName, 'Execute a command on each individual file')
            [CompletionResult]::new('-X', 'X', [CompletionResultType]::ParameterName, 'Execute a command on the batch of matching files')
            [CompletionResult]::new('--exec-batch', 'exec-batch', [CompletionResultType]::ParameterName, 'Execute a command on the batch of matching files')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'Search just by tags or along with a tag(s)')
            [CompletionResult]::new('--tags', 'tags', [CompletionResultType]::ParameterName, 'Search just by tags or along with a tag(s)')
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'No colored output. Should be detected automatically on pipe')
            [CompletionResult]::new('--raw', 'raw', [CompletionResultType]::ParameterName, 'No colored output. Should be detected automatically on pipe')
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'Display only files in the search results')
            [CompletionResult]::new('--only-files', 'only-files', [CompletionResultType]::ParameterName, 'Display only files in the search results')
            [CompletionResult]::new('-G', 'G', [CompletionResultType]::ParameterName, 'Display tags and files on separate lines')
            [CompletionResult]::new('--garrulous', 'garrulous', [CompletionResultType]::ParameterName, 'Display tags and files on separate lines')
            [CompletionResult]::new('-a', 'a', [CompletionResultType]::ParameterName, 'Files matching all tags (instead of any)')
            [CompletionResult]::new('--all', 'all', [CompletionResultType]::ParameterName, 'Files matching all tags (instead of any)')
            [CompletionResult]::new('-A', 'A', [CompletionResultType]::ParameterName, 'Files matching all and only all tags')
            [CompletionResult]::new('--only-all', 'only-all', [CompletionResultType]::ParameterName, 'Files matching all and only all tags')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            break
        }
        'wutag;cp' {
            [CompletionResult]::new('-G', 'G', [CompletionResultType]::ParameterName, 'Use a glob to match files (must be global)')
            [CompletionResult]::new('--glob', 'glob', [CompletionResultType]::ParameterName, 'Use a glob to match files (must be global)')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            break
        }
        'wutag;view' {
            [CompletionResult]::new('-e', 'e', [CompletionResultType]::ParameterName, 'Open tags in selected edtor (use only with vi, vim, neovim)')
            [CompletionResult]::new('--editor', 'editor', [CompletionResultType]::ParameterName, 'Open tags in selected edtor (use only with vi, vim, neovim)')
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'Format of file to view results (toml, yaml, json)')
            [CompletionResult]::new('--format', 'format', [CompletionResultType]::ParameterName, 'Format of file to view results (toml, yaml, json)')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'Search with a tag as a filter')
            [CompletionResult]::new('--tags', 'tags', [CompletionResultType]::ParameterName, 'Search with a tag as a filter')
            [CompletionResult]::new('-p', 'p', [CompletionResultType]::ParameterName, 'Pattern to search for and open result in editor')
            [CompletionResult]::new('--pattern', 'pattern', [CompletionResultType]::ParameterName, 'Pattern to search for and open result in editor')
            [CompletionResult]::new('-a', 'a', [CompletionResultType]::ParameterName, 'a')
            [CompletionResult]::new('--all', 'all', [CompletionResultType]::ParameterName, 'all')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            break
        }
        'wutag;edit' {
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'Set the color of the tag to the specified color. Accepted values are hex colors like ''0x000000'' or ''#1F1F1F'' or just plain ''ff000a''. The colors are case insensitive meaning ''1f1f1f'' is equivalent to ''1F1F1F''')
            [CompletionResult]::new('--color', 'color', [CompletionResultType]::ParameterName, 'Set the color of the tag to the specified color. Accepted values are hex colors like ''0x000000'' or ''#1F1F1F'' or just plain ''ff000a''. The colors are case insensitive meaning ''1f1f1f'' is equivalent to ''1F1F1F''')
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'New name to replace tag with')
            [CompletionResult]::new('--rename', 'rename', [CompletionResultType]::ParameterName, 'New name to replace tag with')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            break
        }
        'wutag;info' {
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'TO BE IMPLEMENTED Do not use color in output')
            [CompletionResult]::new('--raw', 'raw', [CompletionResultType]::ParameterName, 'TO BE IMPLEMENTED Do not use color in output')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            break
        }
        'wutag;repair' {
            [CompletionResult]::new('-m', 'm', [CompletionResultType]::ParameterName, 'Manually set the file''s new location')
            [CompletionResult]::new('--manual', 'manual', [CompletionResultType]::ParameterName, 'Manually set the file''s new location')
            [CompletionResult]::new('-u', 'u', [CompletionResultType]::ParameterName, 'Update the hashsum of all files, including unmodified files')
            [CompletionResult]::new('--unmodified', 'unmodified', [CompletionResultType]::ParameterName, 'Update the hashsum of all files, including unmodified files')
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'Do not actually update the registry')
            [CompletionResult]::new('--dry-run', 'dry-run', [CompletionResultType]::ParameterName, 'Do not actually update the registry')
            [CompletionResult]::new('-R', 'R', [CompletionResultType]::ParameterName, 'Remove files from the registry that no longer exist on the system')
            [CompletionResult]::new('--remove', 'remove', [CompletionResultType]::ParameterName, 'Remove files from the registry that no longer exist on the system')
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'Restrict the repairing to the current directory, or the path given with -d')
            [CompletionResult]::new('--restrict', 'restrict', [CompletionResultType]::ParameterName, 'Restrict the repairing to the current directory, or the path given with -d')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            break
        }
        'wutag;print-completions' {
            [CompletionResult]::new('--shell', 'shell', [CompletionResultType]::ParameterName, 'Shell to print completions. Available shells are: bash, elvish, fish, powershell, zsh')
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'Directory to output completions to')
            [CompletionResult]::new('--dir', 'dir', [CompletionResultType]::ParameterName, 'Directory to output completions to')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            break
        }
        'wutag;clean-cache' {
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            break
        }
        'wutag;ui' {
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
