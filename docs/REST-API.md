# Local application API

HOTR-08 serves HTTP/1.1 on `127.0.0.1:47821` by default. A requested port must be
available; startup never silently selects a different one. Each saved credential
records the actual port. Start with `hotr serve <vault>` and unlock through the
owner CLI. The HTTP listener returns `locked` until the owner unlocks.

Use `hotr request --credential <credential-file>` to check your app's access.
This reads the protected credential, checks the connected server's Windows
identity, and sends one request. It has no proxy, redirect, or automatic retry.
It prints response JSON to stdout; errors also produce a nonzero exit code.
Do not redirect private context into an unprotected log.

For an exact lookup, pipe a JSON object such as
`{"namespace":"project/demo","id":"roadmap"}` into
`hotr request --credential <credential-file> --method POST --endpoint /v1/records/get`.
POST reads at most 256 KiB from stdin before sending. Use UTF-8 JSON and close the
input stream. This command does not accept a token or the vault passphrase.

| Method/path | JSON request | Successful response |
|---|---|---|
| GET `/v1/status` | Empty | State, own client ID/role, schema version |
| POST `/v1/records/get` | `namespace`, `id`, optional positive `revision` | Sourced current or selected historical revision |
| POST `/v1/records` | `record`, nullable `expected_revision`, `idempotency_key` | Durable receipt and replay status |

Example creation document (replace only synthetic values for a first test):

```json
{
  "record": {
    "namespace": "project/demo",
    "id": "roadmap",
    "kind": "roadmap",
    "state": "proposed",
    "body": "First verify the encrypted backup, then connect a second app.",
    "sources": [{"label": "Owner plan", "reference": "local-note:demo"}],
    "tags": ["prototype"]
  },
  "expected_revision": null,
  "idempotency_key": "demo-create-1"
}
```

Use `/v1/records` again with the expected current revision to revise a proposed
record. After a timeout, resend the identical body and idempotency key. A changed
body with that key is a conflict. A timeout or lost connection can mean the commit
succeeded; it is never reported as a definite rollback. Source references are
opaque data and are never fetched. All record and nested request types reject
unknown fields. See RECORD-SCHEMA and TRANSACTIONS for storage semantics.

Rust integrations can call
`hotr::api::scoped_request(&profile, "POST", "/v1/records/get", Some(&json))` after
`hotr::credentials::load(path)`. The running-process test exercises this same
client for status, create, and get. Direct third-party bearer HTTP clients must
provide an equivalent trusted-server boundary; loopback alone does not prove
which Windows account owns the endpoint. Use the supplied client/bridge seam.

Wire requests require exactly one `Host: 127.0.0.1:<port>` and one valid bearer
credential. Any Origin header, absolute-form URL, query string, or wrong Host is
rejected. There are no owner HTTP routes, CORS grants, redirects, SQL endpoint,
shell endpoint, or outbound source fetches. The service rechecks credentials in
the worker after parsing each request, including keep-alive requests.

| Bound | Value |
|---|---|
| Request body / record body | 256 KiB / 64 KiB |
| JSON nesting / path bytes | 32 / 1,024 |
| Header count / header buffer | 32 / 8 KiB |
| Response bytes | 1 MiB |
| Open connections / active handlers | 128 / 64 |
| Queued database work | 256 plus active work |
| Header / handler deadline | 5 seconds / 10 seconds |

Slow headers are closed (or receive HTTP 408 from the parser). Slow bodies hit
the handler deadline and receive 504 with connection close. Known oversized
Content-Length receives 413 before upload; a caller continuing a refused upload
may observe a connection reset. Streaming bodies have the same size ceiling.
Request overload returns 429; connection overload returns bounded 503. The
server uses bounded resources even when a caller does not read a response.

Application responses carry `Cache-Control: no-store`, `nosniff`, and
`Referrer-Policy: no-referrer`. JSON errors contain a stable `error.code` without
internal stack traces, SQL, keys, or request bodies. HTTP status mapping: 400
invalid request, 401 unknown/revoked credential, 403 denied grant/role/Host/Origin,
404 missing record or route, 409 revision/idempotency conflict, 413 size limit,
415 JSON required, 429 overload, 503 locked, and 504 outcome unknown. HTTP parser
errors can close before an application JSON response exists.

Search, MCP, backups, named app integrations, semantic retrieval, packaging, and
non-Windows support are later PSPR gates. This API does not automatically attach
itself to existing applications or authorize private-context provider calls.
