# poppy

This is a simple library which can be used to implement no-limit Texas Hold'em poker gameplay in rust.
Originally built on top of [rs_poker](https://crates.io/crates/rs-poker) the projects diverged quite a lot eventually resulting in a stand-alone library.
There are no dependencies required, though adding serialization support is planned (as a feature).

The gameplay is built as an iterator.
The main design goals were a) being able to present only the valid actions at each point in time to each player b) eventually being able to support simple logging functionality and c) being able to replay rounds starting at any point in time with different players etc.

There is a simple [demo](demo/) which is using this library in a WebAssembly environment.
You can play against a couple of *smart* ai-players - each of them will choose actions randomly.

You can also take a look at the example usage below:
```rust


```

