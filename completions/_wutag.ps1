
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
                $element.Value.StartsWith('-')) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'wutag' {
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'Specify starting path for filesystem traversal')
            [CompletionResult]::new('--dir', 'dir', [CompletionResultType]::ParameterName, 'Specify starting path for filesystem traversal')
            [CompletionResult]::new('-m', 'm', [CompletionResultType]::ParameterName, 'Increase maximum recursion depth from 2')
            [CompletionResult]::new('--max-depth', 'max-depth', [CompletionResultType]::ParameterName, 'Increase maximum recursion depth from 2')
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
            [CompletionResult]::new('--case_insensitive', 'case_insensitive', [CompletionResultType]::ParameterName, 'Case insensitively search')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'Case sensitively search')
            [CompletionResult]::new('--case_sensitive', 'case_sensitive', [CompletionResultType]::ParameterName, 'Case sensitively search')
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'Search with a regular expressions')
            [CompletionResult]::new('--regex', 'regex', [CompletionResultType]::ParameterName, 'Search with a regular expressions')
            [CompletionResult]::new('-g', 'g', [CompletionResultType]::ParameterName, 'Apply operation to all tags and files instead of locally')
            [CompletionResult]::new('--global', 'global', [CompletionResultType]::ParameterName, 'Apply operation to all tags and files instead of locally')
            [CompletionResult]::new('-l', 'l', [CompletionResultType]::ParameterName, 'Respect ''LS_COLORS'' environment variable when coloring the output')
            [CompletionResult]::new('--ls-colors', 'ls-colors', [CompletionResultType]::ParameterName, 'Respect ''LS_COLORS'' environment variable when coloring the output')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'Lists all available tags or files')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Set tag(s) on files that match the given pattern')
            [CompletionResult]::new('rm', 'rm', [CompletionResultType]::ParameterValue, 'Remove tag(s) from the files that match the provided pattern')
            [CompletionResult]::new('clear', 'clear', [CompletionResultType]::ParameterValue, 'Clears all tags of the files that match the provided pattern')
            [CompletionResult]::new('search', 'search', [CompletionResultType]::ParameterValue, 'Searches for files that have all of the provided ''tags''')
            [CompletionResult]::new('cp', 'cp', [CompletionResultType]::ParameterValue, 'Copies tags from the specified file to files that match a pattern')
            [CompletionResult]::new('view', 'view', [CompletionResultType]::ParameterValue, 'View the results in an editor (optional pattern)')
            [CompletionResult]::new('edit', 'edit', [CompletionResultType]::ParameterValue, 'Edits a tag''s color')
            [CompletionResult]::new('print-completions', 'print-completions', [CompletionResultType]::ParameterValue, 'Prints completions for the specified shell to dir or stdout')
            [CompletionResult]::new('clean-cache', 'clean-cache', [CompletionResultType]::ParameterValue, 'Clean the cached tag registry')
            [CompletionResult]::new('ui', 'ui', [CompletionResultType]::ParameterValue, 'Open a TUI to manage tags, requires results from a `search`, or `list`')
            break
        }
        'wutag;list' {
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'If provided output will be raw so that it can be easily piped to other commands')
            [CompletionResult]::new('--raw', 'raw', [CompletionResultType]::ParameterName, 'If provided output will be raw so that it can be easily piped to other commands')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            [CompletionResult]::new('tags', 'tags', [CompletionResultType]::ParameterValue, 'tags')
            [CompletionResult]::new('files', 'files', [CompletionResultType]::ParameterValue, 'files')
            break
        }
        'wutag;list;tags' {
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Print version information')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'c')
            [CompletionResult]::new('--completions', 'completions', [CompletionResultType]::ParameterName, 'completions')
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
        'wutag;rm' {
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            break
        }
        'wutag;clear' {
            [CompletionResult]::new('-n', 'n', [CompletionResultType]::ParameterName, 'Clear all files from registry that no longer exist (requires --global)')
            [CompletionResult]::new('--non-existent', 'non-existent', [CompletionResultType]::ParameterName, 'Clear all files from registry that no longer exist (requires --global)')
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
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'If provided output will be raw so that it can be easily piped to other commands')
            [CompletionResult]::new('--raw', 'raw', [CompletionResultType]::ParameterName, 'If provided output will be raw so that it can be easily piped to other commands')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Display debugging messages on 4 levels (i.e., -vv..)')
            break
        }
        'wutag;cp' {
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
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'The tag to edit')
            [CompletionResult]::new('--tag', 'tag', [CompletionResultType]::ParameterName, 'The tag to edit')
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
