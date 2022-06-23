# Waveling Language

NOTE: this isn't complete or likely to stay accurate for long.  It's just the initial spec so that I can write the
frontend without making obvious mistakes.

Inparticular I am not yet bothering to maintain a list of keywords.

# lexical Structure

## Identifiers

Identifiers are ascii letters, numbers, and `_`, but must start with a letter.

## Boolean Literals

are `true` or `false`.

## Numeric Literals

Numeric literals are written `1`, or `1f64`, or `0xffi64`.

## Comments

Are `//`.

## Model of Computation

Waveling is a language for describing a graph of connected mathematical components which will be run over an input array
once for each element with the explicit goal of being used for audio synthesis and effects, and is most closely thought
of as reactive programming.  Imperative constructions do not exist in the compiled code, only at compile-time for ease
of constructing more complex graphs.  The output of a Waveling compilation invocation is a program which encapsulates
the mathematical expression so described.  For example, the following fragment:

```
output[0] <- input[0] + input[1];
```

Describes an expression which will place in the program's first output array the values of pointwise summing the two
input arrays.

As will be described below, it is possible to maintain state between steps in the implicit outer loop and in so doing to
do things like biquad filters, but it is not possible to control the rate of the loop: Waveling code is only aware that
there is a current sample, and cannot read into the past or future without explicitly maintaining state itself.  Put
another way, if the Waveling program wants to know about the sample 5 samples ago, it must maintain a delay line of 5
samples itself.

Waveling provides a set of primitive operations which implement more complex behaviors over buffers, for example rms or
convolution, where the overhead of expressing the program in Waveling would be too great, or where the subcomponent
doesn't fit the model at all.  For example, the Waveling language itself cannot express FFT convolution or, indeed, even
taking the FFT itself, so such things are provided by the compiler.

The Waveling conceptual execution model always executes all compiled nodes.  The Waveling equivalent of `if`, `select`,
executes both inputs before picking one as the output value.  The compiler takes on the burden of working out if and/or
when it is possible to optimize.  This replaces code like the following:

```
if (time_)to_do_expensive_filter_design) {
    ...
}

run_the_filter();
```

With compiler transformations and user-provided hints, as described later.

## Everything is Signals

The expression:

```
1 + 2;
```

Is a graph, which feeds two constant signals to a `+` node.  It also doesn't compile because the types of the constants
are unknown, see below.  The output shape of this graph is a bundle of one numbered output of one channel.

there is no concept of an expression as anything but a connection between signals.  The constant folding passes will
fold the graph down so that these extra signals disappear.

## Types

Waveling has a set of core types for dealing with data, each building on the last:

- A Primitive is one of `f32`, `f64`, `i32, `i64`, or `bool`.  Primitives never exist on their own.
- A vector is a fixed number of primmitives, stored adjacent in memory.  A scalar is a 1-element vector.
- A buffer is a fixed number of vectors, stored adjacent in memory.

And a set of types for graphs:

- A node is a piece of architecture like a biquad filter.
- An output bundle is a bundle of outputs, either named or numbered.
- An input bundle is a bundle of inputs, either named or numbered.

Informally, a bundle is alike a JavaScript object: a biquad filter might have a bundle of inputs `{0: the_signal, type,
filter, ...}`.

There is no notation for types in user code.  Instead, tuypes are inferred from the graph.  For example, this is an
untyped constant, and can't compile:

```
1 + 2;
```

But this can, because the conversion to `f32` makes the types known:

```
1 + 1 -> f32;
```

## Bundles

A bundle is a collection of pins, either outputs or inputs, to which an endpoint of a connection may be attached (pin is
primarily a term for this document; in code, the distinction is always more clear, but we must be able to speak about
them generically). Each node in the graph has an output bundle and an input bundle, denoted with `.outputs` and
`.inputs`.  Either bundle may be the empty bundle,  `{}`.  A bundle literal:

```
{
    i1,
    i2,
    frequency: modulator,
}
```

Denotes a bundle where i1 and i2 are going to the 0th and 1st pin respectively, and a named pin `frequency` is being fed
from a modulator.

Bundles may be further accessed: `node.inputs.frequency` is the `frequency` pin in the input bundle of `node`.

Neither bundles nor pins may be assined to variables.  Variables always contain full nodes.

The actual semantic meaning of a bundle depends on the built-ins that use it.  This includes whether or not fields are
required, or even if said fields have fixed names.  Since Waveling is a targeted language, we make use of this
flexibility for better offerings in the DSL without having to split the special cases between language elements and
built-in/library-provided pieces.

## Scoping and declaration

Waveling is a lexically scoped language.

variables are introduced with let:

```
let x = 1 + 2 -> f64; // 3, after constant folding.
```

And are valid for their enclosing lexical block.  Names may shadow.

The outer-most scope is the set of built-in and language-provided identifiers.  Nested immediately under this scope is
an implicit scope for the entirety of the program.  That is, a program naming a property the same as a built-in will
refer to the property, not the built-in.  See later on paths.

## Expressions

### basic Operators 

Waveling borrows the following operators from C, with their precedence and associativity from C:

- Arithmetic: `+ - * / %`.
- Bitwise: `~ | & ^ << >>`. - Logical: `! && ||`

`=` is not an operator, it is part of a statement; there is no assignment expression.

### Routing And Routing Operators

Each node in the graph has an input bundle and an output bundle, initially empty.  These are freeform dict/list objects
which can contain anything; the compiler will validate them after the graph is built and error on missing required or
unused keys.  As connections are formed, the bundles expand.  Two non-bool connections to the same key of the same
bundle sum.   two bool connections to the same key of the same bundle logical or, so that event signals can come from
lots of places sensibly.  Two outgoing connections  from the same key of the same bundle splits the data.

Waveling offers  the side-effecting routing operators `->` and `<-`, as the two lowest-precedence operators in the
language.  `->` is higher precedence than `<-`.  Both are left associative.  They do the same thing, but in reverse:

```
// this sends output's bundle to input, then returns input.
input-> output;
// This does the same thing.
output <- input;
```

Which allows doing things like:

```
filtered = audio_file -> biquad -> other_biquad;
filtered -> output[0];
filtered -> reverb -> output[1];
```

The source operand must be an output, a node, or a bundle.  The destination operand must be either an input or a node.
The following are the valid cases, and their meanings:

Source | Destination | Meaning
--- | --- | ---
node | node | Connects the 0th numbered output to the 0th numbered input
node | input | Connects the 0th output of the node to the input.
output | node | Connects the given output to the 0th input
bundle | node | merges the bundle with the node's inputs, so that it is equivalent to having made all the individual connections.
output | input | Connects the given output directly to the input

### Paths

A Path takes the form:

```
[[module]::]variable.[(outputs|inputs)[.pin]]
```

Paths can point at built-ins/functions, nodes, bundles, or outputs/inputs.  Currently, user-defined functions don't
exist, but the compiler offers various built-ins under namespaces (for example, `biquad::lowpass`).


## Statements

There are only 3 kinds of statement: assignment, declaration, and expression:

```
let x = thing; // declaration
x = thing; /// AAssign to x.
a -> b; // Set up routing, but throw out the value.
```

Variables may not be redeclared in the same scope with the same name.

## program Structure

Each program takes the form:

```
program program_name;
external {
    ...
}

stage stage1(an_output=f32(2), another_output=i64(3)) {
    ...
}

stage stage2 {
    ....
}

stage stagen {
    ...
}
```

The meaning of each piece is described below:

### Programm Name

the programm name is a namespace identifier for the generated code.   For example, the C backend prefixes all functions
and types so generated with `program_name_`, so, for example `program_name_run_block`.

### The External Block

The external block connects the program to the external world and takes the form of JSON where quotes are optional on
the keys and also on single-identifier string values (or, put another way, a subset of the JS object syntax).  It must
declare the inputs, outputs, and properties of the program, as well as the samplerate and block size.  For example:

```
external {
    sr: 44100,
    block_size: 128,
    inputs:  [
        { name: the_first_input, width: 1},
        { ... },
    ],
    outputs: [
        { name: the_first_output, width: 1 },
        ...
    ],
    properties: [
        { name: frequency, type: f64 },
    ]
}
```

The inputs, outputs, and propertie arrays introduce nodes into the graph representing the objects in question, under the
given name.  Note that inputs, outputs, and properties are arrays.  This is because external code works with them via
index, not via name; changing the name in the program may or may not break the external API depending on the backend,
but reordering them almost always does.

Inputs and outputs are always f32.  Properties are always f64, but their type is required for future-proofing.

### Stages

After the external block comes the list of stages.  Each stage is a self-contained unit defining a zero-input,
multiple-output node in the graph which will be available to all other stages, with the syntax `stage
stagename(output_name=type(width),...)`.  The stages may occur in any order in the program, though good style is to
write them in the order they can be thought of as running.  The names of all stages except the current stage refer to
the other stages as nodes in the graph.

The reason that stages exist is to allow for vectorization and loop unrolling, and so an additional constraint is placed
on them: any connections of inputs in the current stage to outputs of another stage must be directly to the outputs
declared by that stage.

Within a given stage, the stage's declared outputs are introduced as nodes assigned to the given output names.
