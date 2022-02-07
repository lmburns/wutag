# TODO

## Other

* [ ] Pattern match on full path instead of just file name

## Functions

### Needs built

* [ ] `dir()`
* [ ] `implies()`
* [ ] `implied(<tag>)`
    * [ ] if empty, then any file which has a tag that is implied
    * [ ] or, list tags?
* [ ] `before({a,m,c}time, <val>)`
* [ ] `after({a,m,c}time, <val>)`

#### Maybe

* [ ] `sort()`
    * Id
    * Name
    * ModificationTime
    * CreationTime
    * FileSize
* [ ] `like('%val%')`

### Needs translated into a function

* [ ] `$F`, `$F[1]`, `$F[2..3]`, `$F[2,3]`, `$F[2..=3]`
* [ ] `if a { b }`, `if a { b } else { c }`
* [ ] `unless a { b }`, `unless a { b } else { c }`
* [ ] `a ? b`, `a ? b : c`

### Needs implemented in an API

* [ ] parenthesis order

* [ ] `tag()`
* [ ] `value()`
* [ ] `hash()`

* [ ] `/regex/r`, `%r{regex}`
* [ ] `/glob/g`, `%g{glob}`

* [ ] `not`, `!`
* [ ] `or`, `||`
* [ ] `and`, `&&`
* [ ] `eq`, `==`
* [ ] `ne`, `!=`
* [ ] `gt`, `>`
* [ ] `lt`, `<`
* [ ] `gte`, `>=`
* [ ] `lte`, `<=`
