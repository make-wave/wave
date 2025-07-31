# wave – AI Agent Guide

## Purpose and Use Case

wave is a terminal-based HTTP client designed for developers who prefer working in the terminal. It offers functionality similar to GUI tools like Postman, but with a command-line interface for increased productivity and workflow integration.

## Key Features and Requirements
- Interactive terminal UI for composing and sending HTTP requests
- Support for all major HTTP methods (GET, POST, PUT, DELETE, PATCH, etc.)
- View and edit request/response details (headers, body, status, etc.)
- Save/load collections of requests in YAML format
- CLI commands to generate, edit, and manage collections
- Manual editing of YAML files for advanced users
- Designed for developer productivity and terminal workflow

## Implementation Notes

- **Language & Framework:** wave is built in Rust, using the `clap` package for command-line argument parsing.
- **Testing:** all modules are unit tested to ensure correctness and document behaviour.
- **Output Styling:** Uses [`anstyle`](https://crates.io/crates/anstyle) for colored and bold text, and [`colored_json`](https://crates.io/crates/colored_json) for pretty-printed, colorized JSON output.
- **Command Usage:** Users can invoke HTTP calls directly from the terminal, e.g.:
    - `wave get example.com Authorization:Bearer123 Accept:application/json`
    - `wave post example.com Content-Type:application/json name=joe age=42`
    - `wave put example.com Authorization:Bearer123 foo=bar baz=qux`
    - `wave patch example.com Accept:application/json field=value`
    - `wave delete example.com X-Delete-Reason:cleanup`
    - `wave collectionName requestName` (to run a saved request from a collection)

### Specifying Headers and Body Data
- **Headers:** Use `key:value` syntax anywhere after the URL. Example: `Authorization:Bearer123`
- **Body Data:** Use `key=value` syntax for POST/PUT/PATCH requests. Example: `name=joe`
- The app automatically separates headers from body data.

**Examples:**
```
wave get https://httpbin.org/get Authorization:Bearer123 Accept:application/json
wave post https://httpbin.org/post Content-Type:application/json name=alice age=30
wave put https://httpbin.org/put Authorization:BearerAnother foo=bar
wave patch https://httpbin.org/patch Accept:application/json update=true
wave delete https://httpbin.org/delete X-Delete-Reason:cleanup
```

- **Request Body Handling:**
    - For POST, PUT, and PATCH, use repeated `key=value` arguments after the URL. These are parsed into a JSON object for the request body. Example:
      - `wave post example.com Content-Type:application/json name=joe age=42` sends `{ "name": "joe", "age": "42" }` as JSON.
    - By default, wave sends request bodies as JSON.
- **Collections & Environments:**
    - Collections of requests and environments can be defined in YAML files.
    - wave defaults to checking for YAML configs in a `/.wave/` directory relative to where it was called from.
    - Alternatively, a config path can be specified with a `--config` flag.
- **MCP Tooling Integration:**
    - wave includes MCP tooling so that LLM agents can be given access to use it programmatically.

## Project Structure

wave uses an idiomatic Rust project layout that separates library code from binaries:

- **`src/lib.rs`**: The library root. Contains shared logic, core types, and modules. All reusable functionality is exposed here.
- **`src/bin/`**: The binaries directory. Each file here is a separate CLI tool (binary). The main CLI entrypoint is `src/bin/wave.rs`.
- **`src/printer.rs` and `src/http_client.rs`**: These are modules included in the library via `lib.rs`. They provide output formatting and HTTP client logic, respectively.
- **`src/request_mapper.rs`**: Contains all logic for mapping CLI arguments to HTTP requests. This makes the mapping logic reusable and testable, and keeps the binary thin.

This structure allows:
- Easy reuse of core logic in multiple binaries
- Clean separation between CLI entrypoints and shared functionality
- The ability to add more binaries by simply creating new files in `src/bin/`

**How Rust builds binaries:**
- `cargo run` runs the default binary (if `src/main.rs` exists)
- `cargo run --bin wave` runs `src/bin/wave.rs`
- You can add more binaries (e.g., `src/bin/foo.rs`) and run them with `cargo run --bin foo`

**Why this layout?**
- It’s the recommended way to organize Rust projects that provide both a library and one or more CLI tools.
- It keeps code maintainable, testable, and easy for contributors to navigate.

## Output Behavior

- **Default Output:**
    - Shows bold and colored status line (e.g. "Status: 200"; color depends on status code)
    - Pretty-prints the response body using [`colored_json`](https://crates.io/crates/colored_json) (JSON keys are bright yellow; values are colorized)
    - Manual pretty-printing logic for JSON was removed in favor of `colored_json` for robust formatting
    - Shows `Content-Type` header if body is not JSON or pretty-printing fails
    - Shows all headers if response status is 4xx/5xx (error)
- **Verbose Output (`--verbose`):**
    - Shows bold and colored status line
    - Shows all response headers (each key/value, colorized)
    - Pretty-prints the response body with `colored_json`

## Current Code Structure and Context

- **Modular Design:**
    - `/src/lib.rs`: Library root, exposes all modules and shared logic.
    - `/src/bin/wave.rs`: CLI parsing and application entrypoint (main binary).
    - `/src/printer.rs`: Output formatting and printing logic (colorized, pretty-printed, and testable).
    - `/src/http_client.rs`: HTTP backend abstraction, request/response structs, and dependency-injected client for testability.
    - `/src/request_mapper.rs`: CLI-to-request mapping logic, used by the binary and exposed for reuse/testing.
- **Testability:**
    - HTTP logic uses dependency injection via the `HttpBackend` trait, allowing for mock backends in unit tests.
    - Output formatting is separated into a pure function (`format_response`) for easy assertion in tests.
    - CLI-to-request mapping logic is now testable in isolation.
- **Unit Tests:**
    - `printer.rs` tests cover color formatting, pretty-printing, verbose output, Content-Type display, and error header display.
    - `http_client.rs` tests cover struct construction and mock backend behavior.
    - All tests pass (`cargo test`).
- **How to Run Tests:**
    - From the project root, run: `cargo test --all --color always`
    - All tests should pass with no failures.

## Notes for Future Development
- Research existing terminal HTTP clients for inspiration and best practices
- Design the YAML schema for collections
- Choose libraries/frameworks for terminal UI, HTTP requests, and YAML parsing
- Outline CLI commands and user experience
- Break down implementation into actionable steps

---

This documentation provides context for resuming work on wave in future sessions. All requirements, design considerations, and current implementation details are captured here.
