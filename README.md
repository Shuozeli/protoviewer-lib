# protoviewer-lib

Interactive protobuf binary encoding visualizer. Inspect protobuf wire format with hex view, structure tree, and decoded JSON -- all in the browser via WASM or as a native desktop app.

## Features

- Schema-less protobuf binary walker (no `.proto` file needed)
- Hex view with color-coded regions (tags, varints, fixed-width, length-delimited)
- Structure tree with expandable field hierarchy
- Decoded JSON view
- 8 built-in example templates (Person, Address Book, Order, Sensor Data, etc.)
- Random schema + data generation for exploration
- Shareable permalinks (schema + data encoded in URL)
- Native desktop app (egui) and WASM web app

## Quick Start

```bash
# Build and test
cargo build --workspace
cargo test --workspace

# Run native desktop app
cargo run

# Build for web (WASM)
cd visualizer && trunk build --release --public-url ./
```

## Architecture

MVU (Model-View-Update) pattern:
- `state.rs` -- `AppState`, `Command`, `Effect`, `dispatch()`
- `view.rs` -- `render_view()` extracts egui rendering, emits commands
- `app.rs` -- Thin shell: dispatch loop, effect execution

## License

MIT. See [LICENSE](LICENSE).
