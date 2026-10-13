# Import schema

Profiles can compose schemas via `import_schema` blocks:

```bsol
profile "my.app" {
  version = 2
  import_schema "project.v1" {
    from = file
    path = "schemas/project.v1.bsol"
  }
  import_schema "board.v2" {
    from = git
    url = "https://github.com/org/schemas.git"
    rev = "0123456789abcdef0123456789abcdef01234567"
    path = "board.v2.bsol"
  }
  import_schema "runtime.v1" {
    from = registry
    package = "beskid-schemas"
    version = "1.0.0"
    path = "runtime.v1.bsol"
  }
  rule "root" { ... }
}
```

## Shorthand

Registry imports may use `@pckg/` shorthand in `from`:

```bsol
import_schema "project.v1" {
  from = "@pckg/beskid-schemas/1.0.0/project.v1.bsol"
}
```

## Alias

When the imported profile name collides, set `alias`:

```bsol
import_schema "project.v1" {
  from = file
  path = "vendor/project.v1.bsol"
  alias = "project.v1.vendor"
}
```

## Resolution

The analysis pipeline resolves imports in phase order:

1. `schema.collect`
2. `schema.resolve.file` / `schema.resolve.git` / `schema.resolve.registry`

File resolution is built into `bsol-analysis`. Registry resolution uses [`bsol-beskid-bridge`](../crates/bsol-beskid-bridge/) when a materialized pckg cache root is configured.

## Bsol project packages

`.bproj` manifests with `type = Bsol` export schemas for registry packages:

```bsol
myschemas "1.0.0" {
  name = "myschemas"
  version = "1.0.0"
  type = Bsol
  schemas {
    export "project.v1" {
      profile = "project.v1"
      path = "schemas/project.v1.bsol"
    }
  }
}
```

## Closure budgets

The resolver checks the complete import closure against these default limits:

| Budget | Default | Failure code |
|---|---|---|
| `import_schema` declarations in the closure | 256 | `ImportCountLimit` |
| Import depth below the root profile | 32 | `ImportDepthLimit` |
| Bytes of distinct schema source | 8 MiB | `ResolvedSchemaLimit` |
| Syntax nodes of all schema sources | 100000 | `ResolvedNodeLimit` |

Each profile in the closure must declare exactly one `version` of `1` or `2` (`InvalidProfileVersion`). A source that one canonical identity reaches twice is checked and counted once. A source that imports itself through the active chain fails with `ImportCycle`.
