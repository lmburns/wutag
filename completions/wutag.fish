
complete -c wutag -n "__fish_use_subcommand" -s d -l dir -d 'Specify starting path for filesystem traversal' -r -f -a "(__fish_complete_directories)"
complete -c wutag -n "__fish_use_subcommand" -s m -l max-depth -d 'Set maximum depth to recurse into' -r
complete -c wutag -n "__fish_use_subcommand" -s M -l min-depth -d 'Only show the results starting at a given depth' -r
complete -c wutag -n "__fish_use_subcommand" -s R -l registry -d 'Specify a different registry to use' -r -F
complete -c wutag -n "__fish_use_subcommand" -s c -l color -d 'When to colorize output' -r
complete -c wutag -n "__fish_use_subcommand" -s t -l type -d 'File-type(s) to filter by: f|file, d|directory, l|symlink, e|empty' -r -f -a "{f	,file	,d	,dir	,l	,symlink	,b	,block	,c	,char	,s	,socket	,p	,fifo	,x	,executable	,e	,empty	}"
complete -c wutag -n "__fish_use_subcommand" -s e -l ext -d 'Filter results by file extension' -r
complete -c wutag -n "__fish_use_subcommand" -s E -l exclude -d 'Exclude results that match pattern' -r -f -a "(__fish_complete_directories)"
complete -c wutag -n "__fish_use_subcommand" -s h -l help -d 'Print help information'
complete -c wutag -n "__fish_use_subcommand" -s V -l version -d 'Print version information'
complete -c wutag -n "__fish_use_subcommand" -s v -l verbose -d 'Display debugging messages on 4 levels (i.e., -vv..)'
complete -c wutag -n "__fish_use_subcommand" -s p -l prune -d 'Do not traverse into directories that match the query'
complete -c wutag -n "__fish_use_subcommand" -s i -l case-insensitive -d 'Case insensitively search'
complete -c wutag -n "__fish_use_subcommand" -s s -l case-sensitive -d 'Case sensitively search'
complete -c wutag -n "__fish_use_subcommand" -s r -l regex -d 'Search with a regular expressions'
complete -c wutag -n "__fish_use_subcommand" -s G -l glob -d 'Search with a glob pattern'
complete -c wutag -n "__fish_use_subcommand" -s F -l fixed-string -d 'Search with a literal fixed-string'
complete -c wutag -n "__fish_use_subcommand" -s g -l global -d 'Apply operation to all tags and files instead of locally'
complete -c wutag -n "__fish_use_subcommand" -s L -l follow -d 'Follow symlinks when performing an action on a file'
complete -c wutag -n "__fish_use_subcommand" -l no-follow -d 'Do not follow symlinks when performing an action on a file'
complete -c wutag -n "__fish_use_subcommand" -s l -l ls-colors -d 'Respect \'LS_COLORS\' environment variable when coloring the output'
complete -c wutag -n "__fish_use_subcommand" -s q -l quiet -d 'Do not display any output for a command'
complete -c wutag -n "__fish_use_subcommand" -f -a "list" -d 'Lists all available tags or files'
complete -c wutag -n "__fish_use_subcommand" -f -a "set" -d 'Set tag(s) and/or value(s) on results from a patterned query'
complete -c wutag -n "__fish_use_subcommand" -f -a "rm" -d 'Remove tag(s) from the files that match the provided pattern'
complete -c wutag -n "__fish_use_subcommand" -f -a "clear" -d 'Clears all tags of the files that match the provided pattern'
complete -c wutag -n "__fish_use_subcommand" -f -a "search" -d 'Searches for files that have all of the provided \'tags\''
complete -c wutag -n "__fish_use_subcommand" -f -a "cp" -d 'Copies tags from the specified file to files that match a pattern'
complete -c wutag -n "__fish_use_subcommand" -f -a "view" -d 'View the results in an editor (optional pattern)'
complete -c wutag -n "__fish_use_subcommand" -f -a "edit" -d 'Edits a tag\'s color'
complete -c wutag -n "__fish_use_subcommand" -f -a "info" -d 'Display information about the wutag environment'
complete -c wutag -n "__fish_use_subcommand" -f -a "repair" -d 'Repair broken/missing/modified files in the registry'
complete -c wutag -n "__fish_use_subcommand" -f -a "print-completions" -d 'Prints completions for the specified shell to dir or stdout'
complete -c wutag -n "__fish_use_subcommand" -f -a "clean-cache" -d 'Clean the cached tag registry'
complete -c wutag -n "__fish_use_subcommand" -f -a "ui" -d 'Open a TUI to manage tags'
complete -c wutag -n "__fish_seen_subcommand_from list; and not __fish_seen_subcommand_from tags; and not __fish_seen_subcommand_from files" -s r -l raw -d 'Output will not be colorized'
complete -c wutag -n "__fish_seen_subcommand_from list; and not __fish_seen_subcommand_from tags; and not __fish_seen_subcommand_from files" -s h -l help -d 'Print help information'
complete -c wutag -n "__fish_seen_subcommand_from list; and not __fish_seen_subcommand_from tags; and not __fish_seen_subcommand_from files" -s v -l verbose -d 'Display debugging messages on 4 levels (i.e., -vv..)'
complete -c wutag -n "__fish_seen_subcommand_from list; and not __fish_seen_subcommand_from tags; and not __fish_seen_subcommand_from files" -s g -l global -d 'Apply operation to all tags and files instead of locally'
complete -c wutag -n "__fish_seen_subcommand_from list; and not __fish_seen_subcommand_from tags; and not __fish_seen_subcommand_from files" -f -a "tags" -d 'List the `Tags` within the database'
complete -c wutag -n "__fish_seen_subcommand_from list; and not __fish_seen_subcommand_from tags; and not __fish_seen_subcommand_from files" -f -a "files" -d 'List the `Files` within the database'
complete -c wutag -n "__fish_seen_subcommand_from list; and __fish_seen_subcommand_from tags" -s V -l with-values -d 'Display values along with the tags'
complete -c wutag -n "__fish_seen_subcommand_from list; and __fish_seen_subcommand_from tags" -s c -l no-count -d 'Do not display tag count'
complete -c wutag -n "__fish_seen_subcommand_from list; and __fish_seen_subcommand_from tags" -s u -l unique -d 'Only display unique occurrences. (See --help)'
complete -c wutag -n "__fish_seen_subcommand_from list; and __fish_seen_subcommand_from tags" -s s -l sort -d 'Sort the tag output. This is more limited than listing files'
complete -c wutag -n "__fish_seen_subcommand_from list; and __fish_seen_subcommand_from tags" -s 1 -l one-per-line -d 'Display one tag per line instead of tags on files'
complete -c wutag -n "__fish_seen_subcommand_from list; and __fish_seen_subcommand_from tags" -s b -l border -d 'Use border separators when formatting output'
complete -c wutag -n "__fish_seen_subcommand_from list; and __fish_seen_subcommand_from tags" -s h -l help -d 'Print help information'
complete -c wutag -n "__fish_seen_subcommand_from list; and __fish_seen_subcommand_from tags" -s v -l verbose -d 'Display debugging messages on 4 levels (i.e., -vv..)'
complete -c wutag -n "__fish_seen_subcommand_from list; and __fish_seen_subcommand_from tags" -s g -l global -d 'Apply operation to all tags and files instead of locally'
complete -c wutag -n "__fish_seen_subcommand_from list; and __fish_seen_subcommand_from files" -s s -l sort -d 'Sort the file paths. See --help for all ways to sort' -r
complete -c wutag -n "__fish_seen_subcommand_from list; and __fish_seen_subcommand_from files" -s t -l with-tags -d 'Display tags along with the files'
complete -c wutag -n "__fish_seen_subcommand_from list; and __fish_seen_subcommand_from files" -s V -l with-values -d 'Display values along with the tags'
complete -c wutag -n "__fish_seen_subcommand_from list; and __fish_seen_subcommand_from files" -s f -l format -d 'Format the tags and files output into columns'
complete -c wutag -n "__fish_seen_subcommand_from list; and __fish_seen_subcommand_from files" -s b -l border -d 'Use border separators when formatting output'
complete -c wutag -n "__fish_seen_subcommand_from list; and __fish_seen_subcommand_from files" -s G -l garrulous -d 'Display tags and files on separate lines'
complete -c wutag -n "__fish_seen_subcommand_from list; and __fish_seen_subcommand_from files" -s r -l relative -d 'Display paths relative to current directory (requires --global)'
complete -c wutag -n "__fish_seen_subcommand_from list; and __fish_seen_subcommand_from files" -s d -l duplicates -d 'Show duplicate file entries'
complete -c wutag -n "__fish_seen_subcommand_from list; and __fish_seen_subcommand_from files" -s h -l help -d 'Print help information'
complete -c wutag -n "__fish_seen_subcommand_from list; and __fish_seen_subcommand_from files" -s v -l verbose -d 'Display debugging messages on 4 levels (i.e., -vv..)'
complete -c wutag -n "__fish_seen_subcommand_from list; and __fish_seen_subcommand_from files" -s g -l global -d 'Apply operation to all tags and files instead of locally'
complete -c wutag -n "__fish_seen_subcommand_from set" -s C -l color -d 'Explicitly select color for tag' -r
complete -c wutag -n "__fish_seen_subcommand_from set" -s p -l pairs -d 'Specify any number of tag=value pairs' -r
complete -c wutag -n "__fish_seen_subcommand_from set" -s V -l value -d 'Specify a value to set all the tag(s) to' -r
complete -c wutag -n "__fish_seen_subcommand_from set" -s c -l clear -d 'Clear the tags on the match(es) before the new one(s) are set'
complete -c wutag -n "__fish_seen_subcommand_from set" -s s -l stdin -d 'Arguments are expected to be passed through stdin'
complete -c wutag -n "__fish_seen_subcommand_from set" -s h -l help -d 'Print help information'
complete -c wutag -n "__fish_seen_subcommand_from set" -s v -l verbose -d 'Display debugging messages on 4 levels (i.e., -vv..)'
complete -c wutag -n "__fish_seen_subcommand_from set" -s g -l global -d 'Apply operation to all tags and files instead of locally'
complete -c wutag -n "__fish_seen_subcommand_from rm" -s p -l pairs -d 'Specify any number of tag=value pairs to delete' -r
complete -c wutag -n "__fish_seen_subcommand_from rm" -s V -l values -d 'Indicate the item(s) in the given list are values instead of tags'
complete -c wutag -n "__fish_seen_subcommand_from rm" -s h -l help -d 'Print help information'
complete -c wutag -n "__fish_seen_subcommand_from rm" -s v -l verbose -d 'Display debugging messages on 4 levels (i.e., -vv..)'
complete -c wutag -n "__fish_seen_subcommand_from rm" -s g -l global -d 'Apply operation to all tags and files instead of locally'
complete -c wutag -n "__fish_seen_subcommand_from clear" -s V -l values -d 'Clear values from files instead of tags'
complete -c wutag -n "__fish_seen_subcommand_from clear" -s h -l help -d 'Print help information'
complete -c wutag -n "__fish_seen_subcommand_from clear" -s v -l verbose -d 'Display debugging messages on 4 levels (i.e., -vv..)'
complete -c wutag -n "__fish_seen_subcommand_from clear" -s g -l global -d 'Apply operation to all tags and files instead of locally'
complete -c wutag -n "__fish_seen_subcommand_from search" -s x -l exec -d 'Execute a command on each individual file' -r -f -a "(__fish_complete_command)"
complete -c wutag -n "__fish_seen_subcommand_from search" -s X -l exec-batch -d 'Execute a command on the batch of matching files' -r -f -a "(__fish_complete_command)"
complete -c wutag -n "__fish_seen_subcommand_from search" -s t -l tags -d 'Search just by tags or along with a tag(s)' -r
complete -c wutag -n "__fish_seen_subcommand_from search" -s r -l raw -d 'No colored output. Should be detected automatically on pipe'
complete -c wutag -n "__fish_seen_subcommand_from search" -s f -l only-files -d 'Display only files in the search results'
complete -c wutag -n "__fish_seen_subcommand_from search" -s G -l garrulous -d 'Display tags and files on separate lines'
complete -c wutag -n "__fish_seen_subcommand_from search" -s a -l all -d 'Files matching all tags (instead of any)'
complete -c wutag -n "__fish_seen_subcommand_from search" -s A -l only-all -d 'Files matching all and only all tags'
complete -c wutag -n "__fish_seen_subcommand_from search" -s V -l with-values -d 'Display values along with the tags'
complete -c wutag -n "__fish_seen_subcommand_from search" -s h -l help -d 'Print help information'
complete -c wutag -n "__fish_seen_subcommand_from search" -s v -l verbose -d 'Display debugging messages on 4 levels (i.e., -vv..)'
complete -c wutag -n "__fish_seen_subcommand_from search" -s g -l global -d 'Apply operation to all tags and files instead of locally'
complete -c wutag -n "__fish_seen_subcommand_from cp" -s t -l tag -d 'Specify an individual tag to copy to the matching file(s)' -r
complete -c wutag -n "__fish_seen_subcommand_from cp" -s p -l pairs -d 'Specify any number of tag=value pairs' -r
complete -c wutag -n "__fish_seen_subcommand_from cp" -s G -l glob -d 'Use a glob to match files (must be global)'
complete -c wutag -n "__fish_seen_subcommand_from cp" -s h -l help -d 'Print help information'
complete -c wutag -n "__fish_seen_subcommand_from cp" -s v -l verbose -d 'Display debugging messages on 4 levels (i.e., -vv..)'
complete -c wutag -n "__fish_seen_subcommand_from cp" -s g -l global -d 'Apply operation to all tags and files instead of locally'
complete -c wutag -n "__fish_seen_subcommand_from view" -s e -l editor -d 'Open tags in selected edtor (use only with vi, vim, neovim)' -r
complete -c wutag -n "__fish_seen_subcommand_from view" -s f -l format -d 'Format of file to view results (toml, yaml, json)' -r -f -a "{toml	,yaml	,yml	,json	}"
complete -c wutag -n "__fish_seen_subcommand_from view" -s t -l tags -d 'Search with a tag as a filter' -r
complete -c wutag -n "__fish_seen_subcommand_from view" -s p -l pattern -d 'Pattern to search for and open result in editor' -r
complete -c wutag -n "__fish_seen_subcommand_from view" -s a -l all -d 'View all tags'
complete -c wutag -n "__fish_seen_subcommand_from view" -s h -l help -d 'Print help information'
complete -c wutag -n "__fish_seen_subcommand_from view" -s v -l verbose -d 'Display debugging messages on 4 levels (i.e., -vv..)'
complete -c wutag -n "__fish_seen_subcommand_from view" -s g -l global -d 'Apply operation to all tags and files instead of locally'
complete -c wutag -n "__fish_seen_subcommand_from edit" -s c -l color -d 'Set the color of the tag to the specified color. Accepted values are hex colors like \'0x000000\' or \'#1F1F1F\' or just plain \'ff000a\'. The colors are case insensitive meaning \'1f1f1f\' is equivalent to \'1F1F1F\'' -r
complete -c wutag -n "__fish_seen_subcommand_from edit" -s r -l rename -d 'New name to replace tag with' -r
complete -c wutag -n "__fish_seen_subcommand_from edit" -s h -l help -d 'Print help information'
complete -c wutag -n "__fish_seen_subcommand_from edit" -s v -l verbose -d 'Display debugging messages on 4 levels (i.e., -vv..)'
complete -c wutag -n "__fish_seen_subcommand_from edit" -s g -l global -d 'Apply operation to all tags and files instead of locally'
complete -c wutag -n "__fish_seen_subcommand_from info" -s d -l deleted -d 'Show the number of deleted items (see --help for calculation)'
complete -c wutag -n "__fish_seen_subcommand_from info" -s m -l mean -d 'Show the averages for each item'
complete -c wutag -n "__fish_seen_subcommand_from info" -s f -l full -d 'Show everything about the registry (all --flags)'
complete -c wutag -n "__fish_seen_subcommand_from info" -s r -l raw -d 'Do not use color in output'
complete -c wutag -n "__fish_seen_subcommand_from info" -s h -l help -d 'Print help information'
complete -c wutag -n "__fish_seen_subcommand_from info" -s v -l verbose -d 'Display debugging messages on 4 levels (i.e., -vv..)'
complete -c wutag -n "__fish_seen_subcommand_from info" -s g -l global -d 'Apply operation to all tags and files instead of locally'
complete -c wutag -n "__fish_seen_subcommand_from repair" -s m -l manual -d 'Manually set the file\'s new location' -r -F
complete -c wutag -n "__fish_seen_subcommand_from repair" -s u -l unmodified -d 'Update the hash sum of all files, including unmodified files' -r
complete -c wutag -n "__fish_seen_subcommand_from repair" -s d -l dry-run -d 'Do not actually update the registry'
complete -c wutag -n "__fish_seen_subcommand_from repair" -s R -l remove -d 'Remove files from the registry that no longer exist on the system'
complete -c wutag -n "__fish_seen_subcommand_from repair" -s r -l restrict -d 'Restrict the repairing to the current directory, or the path given with -d'
complete -c wutag -n "__fish_seen_subcommand_from repair" -s h -l help -d 'Print help information'
complete -c wutag -n "__fish_seen_subcommand_from repair" -s v -l verbose -d 'Display debugging messages on 4 levels (i.e., -vv..)'
complete -c wutag -n "__fish_seen_subcommand_from repair" -s g -l global -d 'Apply operation to all tags and files instead of locally'
complete -c wutag -n "__fish_seen_subcommand_from print-completions" -l shell -d 'Shell to print completions. Available shells are: bash, elvish, fish, powershell, zsh' -r -f -a "{bash	,zsh	,powershell	,elvish	,fish	}"
complete -c wutag -n "__fish_seen_subcommand_from print-completions" -s d -l dir -d 'Directory to output completions to' -r -f -a "(__fish_complete_directories)"
complete -c wutag -n "__fish_seen_subcommand_from print-completions" -s h -l help -d 'Print help information'
complete -c wutag -n "__fish_seen_subcommand_from print-completions" -s v -l verbose -d 'Display debugging messages on 4 levels (i.e., -vv..)'
complete -c wutag -n "__fish_seen_subcommand_from print-completions" -s g -l global -d 'Apply operation to all tags and files instead of locally'
complete -c wutag -n "__fish_seen_subcommand_from clean-cache" -s h -l help -d 'Print help information'
complete -c wutag -n "__fish_seen_subcommand_from clean-cache" -s v -l verbose -d 'Display debugging messages on 4 levels (i.e., -vv..)'
complete -c wutag -n "__fish_seen_subcommand_from clean-cache" -s g -l global -d 'Apply operation to all tags and files instead of locally'
complete -c wutag -n "__fish_seen_subcommand_from ui" -s h -l help -d 'Print help information'
complete -c wutag -n "__fish_seen_subcommand_from ui" -s v -l verbose -d 'Display debugging messages on 4 levels (i.e., -vv..)'
complete -c wutag -n "__fish_seen_subcommand_from ui" -s g -l global -d 'Apply operation to all tags and files instead of locally'
