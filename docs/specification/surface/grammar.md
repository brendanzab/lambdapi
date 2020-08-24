# Grammar

This defines the grammar of the surface language.

## Terms

> **Grammar**:
>
> <a href="#var:term"><var id="var:term">term</var></a> ::=\
> &emsp;|&ensp;<a href="#var:arrow-term"><var>arrow-term</var></a>\
> &emsp;|&ensp;<a href="#var:arrow-term"><var>arrow-term</var></a> `:` <a href="#var:term"><var>term</var></a>
>
> <a href="#var:arrow-term"><var id="var:arrow-term">arrow-term</var></a> ::=\
> &emsp;|&ensp;<a href="#var:app-term"><var>app-term</var></a>\
> &emsp;|&ensp;<a href="#var:app-term"><var>app-term</var></a> `->` <a href="#var:arrow-term"><var>arrow-term</var></a>
>
> <a href="#var:app-term"><var id="var:app-term">app-term</var></a> ::=\
> &emsp;|&ensp;<a href="#var:atomic-term"><var>atomic-term</var></a>\
> &emsp;|&ensp;<a href="#var:atomic-term"><var>atomic-term</var></a> <a href="#var:atomic-term"><var>atomic-term</var></a><sup>+</sup>
>
> <a href="#var:atomic-term"><var id="var:atomic-term">atomic-term</var></a> ::=\
> &emsp;|&ensp;`(` <a href="#var:term"><var>term</var></a> `)`\
> &emsp;|&ensp;<var>name</var>\
> &emsp;|&ensp;<var>number-literal</var>\
> &emsp;|&ensp;<var>character-literal</var>\
> &emsp;|&ensp;<var>string-literal</var>\
> &emsp;|&ensp;`[` (<a href="#var:term"><var>term</var></a> `,`)<sup>\*</sup> <a href="#var:term"><var>term</var></a><sup>?</sup> `]`\
> &emsp;|&ensp;`Record` `{` (<a href="#var:type-field"><var>type-field</var></a> `,`)<sup>\*</sup> <a href="#var:type-field"><var>type-field</var></a><sup>?</sup>  `}`\
> &emsp;|&ensp;`record` `{` (<a href="#var:term-field"><var>term-field</var></a> `,`)<sup>\*</sup> <a href="#var:term-field"><var>term-field</var></a><sup>?</sup> `}`\
> &emsp;|&ensp;<a href="#var:atomic-term"><var>atomic-term</var></a> `.` <var>name</var>\
> &emsp;|&ensp;<a href="#var:atomic-term"><var>atomic-term</var></a> `^` <var>number-literal</var>

## Fields

> **Grammar**:
>
> <a href="#var:type-field"><var id="var:type-field">type-field</var></a> ::=\
> &emsp;|&ensp;<var>doc-comment</var><sup>\*</sup> <var>name</var> `:` <a href="#var:term"><var>term</var></a>
>
> <a href="#var:term-field"><var id="var:term-field">term-field</var></a> ::=\
> &emsp;|&ensp;<var>doc-comment</var><sup>\*</sup> <var>name</var> `=` <a href="#var:term"><var>term</var></a>
