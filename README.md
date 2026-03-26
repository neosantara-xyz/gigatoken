# Jeton Rust

Jeton is faster than other libraries due to algorithmic and systems changes.
One key benefit of using Jeton is that it replaces the regex expression used by almost all tokenizers to do pre-tokenization with a custom implementation of the exact same method.
This is a serious bottleneck for other implementations, and is a big part in this library's breakneck speeds.
Additionally, Jeton uses concurrent data structures to use multiprocessing in more places.