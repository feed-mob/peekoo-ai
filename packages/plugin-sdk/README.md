# @peekoo/plugin-sdk

AssemblyScript SDK for Peekoo plugins.

Use it as a local path dependency for now:

```json
{
  "dependencies": {
    "@peekoo/plugin-sdk": "file:../../packages/plugin-sdk",
    "@extism/as-pdk": "^1.0.0"
  }
}
```

Example imports:

```ts
import * as state from "@peekoo/plugin-sdk/assembly/state";
import * as log from "@peekoo/plugin-sdk/assembly/log";
import * as schedule from "@peekoo/plugin-sdk/assembly/schedule";
```

See `docs/plugin-authoring.md` for a full plugin authoring walkthrough.
