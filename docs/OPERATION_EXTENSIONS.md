# Operation Extensions

Pasted extensions contribute Operations and supporting providers without
receiving unrestricted access to Pasted internals. The host owns execution,
permissions, secrets, timeouts, output limits, history, and clipboard writes.

## Product boundaries

- **Built-in Operations** ship with Pasted, use stable `builtin:` references,
  and are immutable.
- **Custom Operations** belong to the user, use stable `custom:` references,
  and are editable.
- **Extension Operations** are contributed by a versioned manifest and use
  stable `plugin:<plugin-id>/<operation-id>` references.
- **Credential providers** resolve secret references for executors. They are
  not Operations and never place resolved values in SQLite or manifests.
- **Connections** are user-configured provider accounts. An Operation refers
  to a connection ID rather than embedding a token.

## Initial plugin format

The first plugin format is declarative JSON. It may contribute Operations and
credential providers using executor kinds implemented by the Pasted host.
Arbitrary JavaScript, dynamic native libraries, and unsandboxed Rust plugins
are deliberately excluded from version 1.

Every manifest declares exact capabilities:

- network hosts;
- process executable names;
- permitted secret-provider classes.

Installations begin disabled until the user reviews these capabilities.
Updates that expand permissions require review again. Imported Operations and
connections begin disabled and untrusted.

## Execution rules

Extensions receive an input value and return an output value through the same
canonical execution service as built-ins and manual Transforms. They cannot directly
read clipboard history, query SQLite, mutate Bins, or write to the system
clipboard. Automations remain host-owned and explicitly opt into an extension
Operation.

Execution records store identities, revisions, hashes, duration, status, and a
safe error summary. They never store input text, output text, tokens, or
resolved credentials.

## Example: OpenAI Operations

The OpenAI example contributes Summarize and Rewrite Operations. It declares
network access only to `api.openai.com` and refers to an `openai.default`
connection. The connection owns model selection and a Keychain or credential
provider reference. Pasted sends only the current Operation input after an
explicit run, unless the user separately enables an Automation.

OpenAI's Responses API is the intended host executor. The extension manifest
does not contain an API key and does not choose a permanently hard-coded model.

## Example: 1Password

The 1Password example contributes a credential provider rather than a text
Operation. It resolves `op://` references just in time through the `op` CLI,
with user presence required. The resolved secret is supplied directly to the
request or process environment and is never returned as an Operation output.

This allows an OpenAI or HTTP connection to use 1Password without teaching
every Operation how to authenticate or encouraging secrets to enter clipboard
history.

## Later evolution

After the declarative host-executor model is proven, deterministic pure-text
extensions may gain a WASM executor with strict memory, time, and capability
limits. Broader code-loading should remain unnecessary for most integrations.
