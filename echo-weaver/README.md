# echo-weaver

An auto splitter for
[Echo Weaver](https://store.steampowered.com/app/2184080/Echo_Weaver/).

## Features

- **Auto-Start**: The timer is automatically started on the first input of a
  cycle.
- **Auto-Reset**: The timer is automatically reset when save data is reset. It
  can *optionally* be reset at the start of each cycle (for Any%).
- **Auto-Split**: A split is automatically created when opening the Relic Door
  (game end). Splits can *optionally* be created for various other progression
  events (see [Options](#options)).

### Options

- **Loop Behavior**: Controls behavior when a new cycle is started.
  - **None**: Do nothing.
  - **Split**: Create a new split. (Recommended for 100%)
  - **Reset**: Reset the timer. (Recommended for Any%)
- **Split on Elymere Pumphouse completion**: Splits each time the pumphouse
  puzzle is completed. (Recommended for Any%)
- **Split on mended major rifts**: Splits after mending the rifts at Anchor Node
  Jurat, Mercy, and Vera. (Recommended for Any%)

  *Note: This may also cause a split if the Loom rift is mended without
  previously mending Jurat, Mercy, and Vera. Please create an Issue if this is
  problematic for your routing.*
- **Split on Loom activation**: Splits after activating the loom, just before
  beginning the final gauntlet.

### Custom Variables

The following variables are exposed for use in LiveSplit layouts:

- `loop`: The current cycle number.
- `remnants`: The number of Remnants of a Weaver that have been collected.
- `vestiges`: The number of Vestiges of an Orator that have been collected.
- `dewdrops`: The number of Dewdrops of Enessa that have been collected.
- `enth`: The number of Calcified Enth the player currently holds. **This will
  decrease if Calcified Enth are consumed.**
- `leads`: The number of leads that have been unlocked.
- `collectibles`: All collectible counts formatted as concise string:
  `$remnants/$vestiges/$dewdrops/$enth/$leads`.

**Variables only display information that is available in the Status and System
menus.**

## Compilation

This auto splitter is written in Rust. In order to compile it, you need to
install the Rust compiler:
[Install Rust](https://www.rust-lang.org/tools/install).

Afterwards install the WebAssembly target:

```sh
rustup target add wasm32-unknown-unknown --toolchain stable
```

The auto splitter can now be compiled:

```sh
cargo b --release --no-default-features
```

The auto splitter is then available at:

```none
target/wasm32-unknown-unknown/release/echo_weaver.wasm
```

Make sure to look into the [API documentation](https://livesplit.org/asr/asr/)
for the `asr` crate.

## Development

You can use the [debugger](https://github.com/LiveSplit/asr-debugger) while
developing the auto splitter to more easily see the log messages, statistics,
dump memory, step through the code and more.

The repository comes with preconfigured Visual Studio Code tasks. During
development it is recommended to use the `Debug Echo Weaver` launch action to
run the `asr-debugger`. You need to install the `CodeLLDB` extension to run it.

You can then use the `Build Echo Weaver (Debug)` task to manually build the
auto splitter. This will automatically hot reload the auto splitter in the
`asr-debugger`.

Alternatively you can install the [`cargo
watch`](https://github.com/watchexec/cargo-watch?tab=readme-ov-file#install)
subcommand and run the `Watch Echo Weaver` task for it to automatically build
when you save your changes.

The debugger is able to step through the code. You can set breakpoints in VSCode
and it should stop there when the breakpoint is hit. Inspecting variables may
not work all the time.
