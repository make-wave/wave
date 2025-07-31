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

To build from source:

```sh
cargo install --path .
```

## Usage

Basic request examples:

```sh
wave get https://httpbin.org/get Authorization:Bearer123 Accept:application/json
wave post https://httpbin.org/post Content-Type:application/json name=alice age=30
wave put https://httpbin.org/put Authorization:BearerAnother foo=bar
wave patch https://httpbin.org/patch Accept:application/json update=true
wave delete https://httpbin.org/delete X-Delete-Reason:cleanup
```

- **Headers:** Use `key:value` syntax after the URL (e.g., `Authorization:Bearer123`).
- **Body Data:** For POST/PUT/PATCH, use `key=value` syntax (e.g., `name=alice`). Body data defaults to JSON. Specify form data with `--form`.
- **Collections:** Save requests in YAML files in the `.wave` directory and run them by name: `wave my_collection my_request`

### Example Collection YAML

```yaml
variables:
  base_url: https://api.example.com
  auth_token: secret123
  user_id: 42

requests:
  - name: Get User Info
    method: GET
    url: ${base_url}/users/${user_id}
    headers:
      Authorization: Bearer ${env:API_TOKEN}
      Accept: application/json

  - name: Create User
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
- Run a request with: `wave example_api "Get User Info"`
- The collection name is always the filename (without `.yaml`), not a field in the file.

## Help

Run `wave --help` for full command-line options and usage details.

---

For developer and AI agent documentation, see [AI_AGENT_GUIDE.md](./AI_AGENT_GUIDE.md).
