pub(crate) const ZSH_COMPLETION_REP: &[(&str, &str)] = &[
    (
        "(( $+functions[_wutag__clean-cache_commands] )) ||
_wutag__clean-cache_commands() {
    local commands; commands=()
    _describe -t commands 'wutag clean-cache commands' commands \"$@\"
}
(( $+functions[_wutag__clear_commands] )) ||
_wutag__clear_commands() {
    local commands; commands=()
    _describe -t commands 'wutag clear commands' commands \"$@\"
}
(( $+functions[_wutag__cp_commands] )) ||
_wutag__cp_commands() {
    local commands; commands=()
    _describe -t commands 'wutag cp commands' commands \"$@\"
}
(( $+functions[_wutag__edit_commands] )) ||
_wutag__edit_commands() {
    local commands; commands=()
    _describe -t commands 'wutag edit commands' commands \"$@\"
}
(( $+functions[_wutag__list__files_commands] )) ||
_wutag__list__files_commands() {
    local commands; commands=()
    _describe -t commands 'wutag list files commands' commands \"$@\"
}
(( $+functions[_wutag__list_commands] )) ||
_wutag__list_commands() {
    local commands; commands=(
'tags:' \\
'files:' \\
    )
    _describe -t commands 'wutag list commands' commands \"$@\"
}
(( $+functions[_wutag__print-completions_commands] )) ||
_wutag__print-completions_commands() {
    local commands; commands=()
    _describe -t commands 'wutag print-completions commands' commands \"$@\"
}
(( $+functions[_wutag__rm_commands] )) ||
_wutag__rm_commands() {
    local commands; commands=()
    _describe -t commands 'wutag rm commands' commands \"$@\"
}
(( $+functions[_wutag__search_commands] )) ||
_wutag__search_commands() {
    local commands; commands=()
    _describe -t commands 'wutag search commands' commands \"$@\"
}
(( $+functions[_wutag__set_commands] )) ||
_wutag__set_commands() {
    local commands; commands=()
    _describe -t commands 'wutag set commands' commands \"$@\"
}
(( $+functions[_wutag__list__tags_commands] )) ||
_wutag__list__tags_commands() {
    local commands; commands=()
    _describe -t commands 'wutag list tags commands' commands \"$@\"
}
(( $+functions[_wutag__ui_commands] )) ||
_wutag__ui_commands() {
    local commands; commands=()
    _describe -t commands 'wutag ui commands' commands \"$@\"
}
(( $+functions[_wutag__view_commands] )) ||
_wutag__view_commands() {
    local commands; commands=()
    _describe -t commands 'wutag view commands' commands \"$@\"
}
",
        r#"(( $+functions[_wutag__list_commands] )) ||
_wutag__list_commands() {
    local commands; commands=(
        "tags:" \
        "files:" \
    )
    _describe -t commands 'wutag list commands' commands "$@"
}
(( $+functions[_wutag_tags] )) ||
_wutag_tags() {
    [[ $PREFIX = -* ]] && return 1
    integer ret=1
    local -a tags; wtags=(
        ${(@f)$(_call_program commands wutag -g list -r tags -c)}
    )

    _describe -t wtags 'tags' wtags && ret=0
    return ret
}"#,
    ),
    // Make this replace globally
    (r#"'*::tags:' \"#, r#"'*::_wutag_tags:' \"#),
    (r#"'*::tags:' \"#, r#"'*::tags:_wutag_tags' \"#),
    (
        "'-t+[The tag to edit]:tag: ' \\
'--tag=[The tag to edit]:tag: ' \\",
        "'-t+[The tag to edit]::_wutag_tags' \\
'--tag=[The tag to edit]::_wutag_tags' \\",
    ),
];
