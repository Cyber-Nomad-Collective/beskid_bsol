# Import schema

Profiles can compose schemas via `import_schema` blocks:

```bsol
profile "my.app" {
  import_schema "project.v1" {
    from = file
    path = "schemas/project.v1.bsol"
  }
  import_schema "board.v2" {
    from = git
    url = "https://github.com/org/schemas.git"
    rev = "abc123"
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
