# wave

wave is an HTTP client for folks who like their terminal. It provides a simple, scriptable alternative to GUI tools like Postman, making it easy to send HTTP requests, inspect responses, and automate API workflows directly from your shell. And it's written in Rust!

## Features
- GET, POST, PUT, PATCH, DELETE methods
- Specify headers and body data inline
- Responses printed in an easy-to-read format
- Save and run collections of requests via YAML config files
- Easy integration with other terminal applications
- MCP integration for LLM agents [coming soon!]
- GraphQL requests [coming soon!]

## Installation

To install with homebrew:
```sh
brew tap make-wave/tap
brew install make-wave/tap/wave
```

To build from source:

```sh
cargo install --path .
```

## Usage

Basic request examples:

```sh
wave get https://httpbin.org/get 
wave post https://httpbin.org/post name=alice age=30
wave put https://httpbin.org/put --form Authorization:Bearer123 foo=bar
wave patch https://httpbin.org/patch Accept:application/json update=true
wave delete https://httpbin.org/delete X-Delete-Reason:cleanup
```

- **Headers:** Use `key:value` syntax, e.g. `Authorization:Bearer123`
- **Body Data:** Use `key=value` syntax, e.g. `name=alice`. Defaults to JSON. Specify form data with `--form`. The correct `Content-Type` header is applied automatically.
- **Collections:** Save requests in YAML files in the `.wave` directory and run them by name. E.g. for a request called `my_request` in `.wave/my_collection.yml`: `wave my_collection my_request`

### Example Collection YAML

```yaml
variables:
  base_url: https://api.example.com
  auth_token: secret123
  user_id: 42

requests:
  - name: get-user-info
    method: GET
    url: ${base_url}/users/${user_id}
    headers:
      Authorization: Bearer ${env:API_TOKEN}
      Accept: application/json

  - name: create-user
    method: POST
    url: ${base_url}/users
    headers:
      Authorization: Bearer ${env:API_TOKEN}
      Content-Type: application/json
    body:
      json:
        name: Alice
        email: alice@example.com
```

- Use `${varName}` to reference variables defined in the file.
- Use `${env:VAR_NAME}` to reference environment variables.
- Place your YAML files in the `.wave` directory, e.g., `.wave/example_api.yaml`.
- Run a request with: `wave example_api get-user-info`
- The collection name is the file name (without the extension).

## Help

Run `wave --help` for full command-line options and usage details.

---

For developer and AI agent documentation, see [AI_AGENT_GUIDE.md](./AI_AGENT_GUIDE.md).
