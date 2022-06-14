# Design.

## Introduction

Waveling is a reactive programming language which expresses graphs of primitive audio components.  It does not work at
the per-sample level at least for now, nor will it initially support traditional imperative constructs like if
statements.  Instead, things like that are implemented through such techniques as bitmasks.  Primitives which cannot be
expressed in waveling itself will be added as built-ins, for example convolution engines.

One particular consequence of this design is that, at least at the source level, all signals always execute.  There is
no option in the mvp to prevent this.  In the long run, a basic prevention can be added via compiler passes, or by
adding built-ins (say, `select_lazy(cond, true case, false case)`.

This model is sufficient (though in some cases convoluted) enough to express at least Synthizer's Fdn reverb,
chorus/flangers, and things like basic additive and fm synthesis.

## Types

All types are signals.  We will have (though not all in the mvp):

- `i32, i64`: the integer types.
- `f32, f64`: the floating point types.
- `Signal<I_COUNT, O_COUNT>`: A component (e.g. biquad, buffer reader).
- `frame<T,N>`: a vector of audio samples.
- `input<T,N>`, an input to which something can be attached.
- `output<T,N>`, something which can be attached to an input.
- `mask32, mask64`: a bool, but truth values are all 1s.

Most of this is self-explanatory.  Inputs can connect to outputs if their type and width matches.  Signals can have any
number of inputs and outputs.

What a signal actually is is somewhat opaque at the source code level because waveling will not allow assigning to
variables more than once, and naming nested types is basically impossible.  See below about operations.

Though these syntaxes look like generics, they initially aren't.  They might be in future, but for now they're going to
be language built-ins.

Constants are modeled as signals which are always returning the same value.  That is, any floating point value is a
`Signal<f32, 1>`.

## Promotion/conversion rules.

We will have the following implicit conversion rules:

- `f32` promotes to `f64`.
- `i32` promotes to `i64`.
- So do `mask32, `mask64`.
- Vectors of these types promote to vectors of the "higher" type.
- Signals convert to their first output or first input as needed.
- Outputs promote their types as needed to match inputs they connect to.
- These rules apply recursively.

## Basic operations

### Assignment

Assignment in waveling is actually naming a block, not assigning to a mutable variable.  That is, it's like an alias but
with the additional property that the subgraph only executes once per sample.  SO:

```
filter = biquad(whatever);
thing1 = filter -> whatever
thing2 = filter -> whatever
```

Will evaluate filter once and send the output to two places.

### Basic arithmetic operators

We will adopt all unary and binary operators from C, which will produce signals based on their inputs.  They will
require that the inputs have the same channel counts.

### Comparisons

We will adopt all comparison operators from C, which will output masks.  Specifically, `mask32`, relying on promotions
to get to `mask64` if needed.

### Bitwise operators

We will adopt all bitwise operators from C.

Shifts work on integral types.  They produce a signal with the shifted value.

Binary bitwise operators work either on integral types and masks.  `&` additionally works on `anything OP mask`,
returning `anything` or the zero-value of `anything`.  Examples:

Two integers ored: `a | b`.

Enable a signal only if a comparison returns true: `signal | (other_signal > 0.5)`.

### Comparisons

We again will adopt all comparison operators from C, which will produce masks matching the widths of their inputs.

### Logical operators

These don't exist because the concept of short-circuiting doesn't exist; use the bitwise ones instead.

## Signals

### Introduction

There are two kinds of signals: built-in objects, and combinations of these.  There are two special built-in objects in
scope for every program: `program_input` and `program_output`, which represent the inputs and outputs of the program.
Additionally, the user can add custom-named input and output signals in their script to represent things like
properties.

Every signal has inputs and outputs.

When feeding to a signal's input, all inputs must be connected at once.  See below for how.  This is because it doesn't
make much sense to connect after the fact, given that this is a purely reactive language.

Outputs are referenced by the `[]` operator, `signal[0]` for the first output and so on.  So:

```
signal1 -> signal2`
```

Connects the first output of signal1 to the first input of signal2.

```
signal1[1] -> signal2
```

Connects the second output instead.

### routing operations.

Signal outputs are connected to inputs with `->`, the operator with the lowest precedence in the language.  SO:

```
signal1 + signal2 -> program_output
```

Adds two signals and puts them in the output.

Outputs are stacked for connection with `,`, the second-lowest operator in the language.  So:

```
signal1, signal2 -> program_output
```

Passes signal1 and signal2 to the first and second outputs of the program.

Outputs are broadcasted with the built-in `broadcast` signal and truncated with the `truncate` signal:

- Broadcasting widens an input by adding zero-valued samples.
- Truncating drops extra channels.

Example: `broadcast(signal) -> wider_thing`.

Outputs are merged into a wider output with the built-in `merge` signal, which stacks them one after another and
produces a wider output whose width is the width of the outputs summed: `merge(a, b)`.

And split with `split`, which takes a variadic set of arguments: `split(output, index1, index2, ...)`, so:
`split(output, 2, 4)` grabs the first 2 channels and the second 2 channels of the output, producing two outputs.

Outputs may be sliced with the slicing signal: `slice(output, index1, [index2])`.  This produces a new output with a
subset of the channels, either between the indices or from a given index to the highest channel.

### rates

Every signal has a number of samples after which it changes.  We call this the rate.  We have three special rate names:
s-rate, b-rate, and c-rate.  s-rate signals update on every sample.  b-rate signals update on every program invocation,
called a block (see below about runtime).  c-rate signals are constant, and stay the same for the entire duration of the
program.

We guarantee constant folding of c-rate signals, evaluated at infinite precision.

### Built-ins.

We will offer:

- Built-ins for mathematical constants `PI` and `E`.
- built-ins for all trigonometric operations.
- Built-ins for one-pole, one-zero, and biquad filters.
- Built-ins for signal selection.
- Built-ins for random numbers.

#### Signal selection

We offer two built-ins for signal selection.  `if` and `select`.

`if(condition, true, false)` will return the true branch's current value if the mask returned by the condition is true,
otherwise false.

`select(index, signal1, signal2, [default signal3])` will take an integral index and return the signal's value
corresponding to that index.  Out of range indexes return zero or, if the optional default is provided, the value of
that default.

#### Filters

Filters have one input of a given width, and one output of the same width.  We will expose built-ins for one-zero,
one-pole, and biquad filters, namespaced under the filter type, e.g. `biquad.lowpass`.  The design functions take
signals and produce filters which redesign themselves as needed.  In practice this is almost the same as doing it in the
language, but with the ability to gain extra efficiency.

#### Random numbers

The special built-in `xoroshiro<T,N>` is a random number generator, seeded by the embedder, which yields one output
containing `N` random `T`.  `T` must be a primitive scalar type.

### Basic recursion

We will introduce a syntax for one-sample delayed recursion:

```
cell (start, end): vector<T, N>;
```

Which declares two signals, start and end.  Start is a signal which copies the value from end after end evaluates,
producing it next time.

Additionally:

```
cell(5) (start, end): vector<T, N>;
```

Is a recursion cell, but delaying by 5 samples.  The delay must be an integer literal.


### Buffers and delay lines

Declaring a delay line is done as:

```
buffer foo(10): vector<f32, 2>;
```

Which is a stereo buffer of capacity 5 samples.

To turn this into a delay line, we use the special signals `delread(buffername, delay)` where delay is a signal
producing integer sample values, and `delwrite(buffername, value)` to write it at the current write head position. These
coalesce into a single delay index that increments implicitly by the compiler at every sample advance, so that
`delwrite`s are always writing at the head and `delread`s at the head minus some value.

It is possible to write before reading (in which case the read can be done at index 0, and have a defined value) or to
read before writing (in which case index 0 is some undefined value, probably from the past).

## The runtime

Every program has a specific block size, and executes one block on every call.

Every program has at least one output buffer, which is one block by some number of channels.  These are declared at the
top of the program:

```
outputs n1=2, n2=2...
```

Outputs may be accessed by name or by index.

Every program optionally has some number of buffer-valued inputs:

```
inputs n1=2, n2=2...
```

For the mvp, inputs and outputs are always f32.

Which may also be accessed by index or name, and get filled by the program before the next block.

Finally, every program has properties:

```
property(s|b) name
```


For the mvp these are also only f32.  The rate controls whether the property is a buffer or a single value, though at
runtime we will eventually provide the ability to switch that via the embedder, compiling the program multiple times
with all permutations.
