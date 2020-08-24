---
id: functions
title: Functions (WIP)
keywords:
  - docs
  - reference
  - pikelet
---

A function is a way of relating an input to an output.

## Formation

Function types are written as `A -> B`.
Functions are [_curried_][currying-wikipedia], meaning that they take a single input, and return a single output.
Multi-argument functions can be created by creating functions that output other functions.

For example, the function type for adding two 32-bit signed integers together is:

```pikelet
S32 -> S32 -> S32
```

## Construction

Functions are constructed by specifying a list of one-or-more parameter names after a `fun` token,
and then a body term after a `=>` token.
The parameters can then be referred to in the body of the function.

```pikelet
fun param-1 param-2 => body
```

Note that functions must always be constructed in a position where they can find a type annotation.
For example, the following function is ambiguous:

```pikelet
fun x y => x
```

This, however is not, because the function type pulled from the record annotation:

```pikelet
record {
    const = fun x y => x,
} : Record {
    const : S32 -> String -> S32,
}
```

:::note
These are sometimes called _lambda abstractions_ in type theory,
ore _anonymous functions_ in programming languages.
:::

## Elimination

Functions can be applied to arguments via [_juxtaposition_][juxtaposition-wikipedia].

For example, this is how the identity function might be applied:

```pikelet
id String "hello!"
```

```pikelet
Array 3 String
```

### Computation

:::warning
This section is a work in progress.
:::

[currying-wikipedia]: https://en.wikipedia.org/wiki/Currying
[juxtaposition-wikipedia]: https://en.wikipedia.org/wiki/Juxtaposition#Mathematics
