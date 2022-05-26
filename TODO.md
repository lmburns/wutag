# TODO

## Other

* [ ] Pattern match on full path instead of just file name

## Functions

### Needs built

* [ ] `dir()`
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

### Possibilities

abc

/abc/ri
/abc/gi
%r{abc}
%g{abc}i
hi*

!foo
not foo

a and b
a && b
a or b
a || b

a == b
a eq b
a != b
a ne b
a > b
a gt b
a < b
a lt b
a >= b
a gte b
a <= b
a lte b

foo ? bar
foo ? bar : baz
foo ? bar ? baz : fah : zag

if foo { bar }
if foo { bar } else { baz }
unless foo { bar }
unless foo { bar } else { baz }

@F
@F[..]
@F[1..]
@F[1..3]
@F[1..=3]
@F[..=3]
@F[1]
@F[1,2]

* **Functions dealing with tags specifically**
value()   v()
tag()     t()
implied()
implies()

* **Functions that aren't necessary**
    * *File fields*
        * hash()
        * before(ctime, yesterday)
        * after(mtime, yesterday)
    * *Other*
        * exec()

a\ b
ab\{
'ab{'
