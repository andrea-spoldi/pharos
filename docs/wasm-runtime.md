# WASM Runtime (Experimental)

The WASM runtime is an architectural spike using Wasmtime.

## Status

**Experimental** — not production-ready. API stability is not guaranteed.

## How It Works

1. Widget specifies `runtime: wasm-experimental` in its manifest
2. `wasm_execute` Tauri command loads the `.wasm` module from `spec.wasmPath`
3. Wasmtime instantiates the module
4. The module's `run` export is called if it exists
5. Config JSON is passed as input; output JSON is returned to the frontend

## Host Capabilities (planned)

```rust
http_get(url: String) -> Result<Response>
get_secret(name: String) -> Result<String>    // permission-checked
emit_event(event: String, payload: String) -> Result<()>
log(level: String, message: String) -> Result<()>
```

These are imported by the WASM module and provided by the Rust host.

## Building a WASM Widget

```rust
// Compile with: cargo build --target wasm32-wasip1 --release
#[no_mangle]
pub extern "C" fn run() {
    // widget logic here
}
```

## Non-Goals

- WASM Component Model (future)
- Public WASM marketplace
- Arbitrary filesystem access from WASM
