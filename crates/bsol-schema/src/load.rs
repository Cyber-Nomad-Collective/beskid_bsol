//! Bootstrap loader: parse declarative schema profile documents into [`SchemaProfile`].

use std::collections::HashMap;
use std::path::Path;

use bsol_syntax::{
    parse_bsol_document, BsolBlock, BsolDocument, BsolItem, BsolListItem, BsolValue,
};

use crate::error::BsolError;
use crate::{
    BlockRule, Cardinality, ExtendSpec, FieldConstraints, FieldRule, ImportSchemaSpec,
    ImportSource, KindMatch, LabelRequirement, MigrationRewrite, MigrationSpec,
    MigrationWhenClause, RuleScope, SchemaProfile, ValueType, VariantRule,
};

const EMBEDDED_PROFILES: &[(&str, &str)] = &[
    ("schema.v1", include_str!("../../../schemas/schema.v1.bsol")),
    ("schema.v2", include_str!("../../../schemas/schema.v2.bsol")),
    (
        "project.v1",
        include_str!("../../../schemas/project.v1.bsol"),
    ),
    (
        "project.v2",
        include_str!("../../../schemas/project.v2.bsol"),
    ),
    (
        "workspace.v1",
        include_str!("../../../schemas/workspace.v1.bsol"),
    ),
    (
        "runtime.v1",
        include_str!("../../../schemas/runtime.v1.bsol"),
    ),
    (
        "runtime.v2",
        include_str!("../../../schemas/runtime.v2.bsol"),
    ),
    ("board.v1", include_str!("../../../schemas/board.v1.bsol")),
    ("board.v2", include_str!("../../../schemas/board.v2.bsol")),
    ("board.v3", include_str!("../../../schemas/board.v3.bsol")),
    (
        "shell.pages.v1",
        include_str!("../../../schemas/shell.pages.v1.bsol"),
    ),
    (
        "tools.config.v1",
        include_str!("../../../schemas/tools.config.v1.bsol"),
    ),
    (
        "configuration.v1",
        include_str!("../../../schemas/configuration.v1.bsol"),
    ),
    (
        "configuration.v2",
        include_str!("../../../schemas/configuration.v2.bsol"),
    ),
];

/// Load an embedded schema profile by name (for example `project.v1`).
pub fn load_profile(name: &str) -> Result<SchemaProfile, BsolError> {
    let source = EMBEDDED_PROFILES
        .iter()
        .find(|(id, _)| *id == name)
        .map(|(_, src)| *src)
        .ok_or_else(|| BsolError::UnknownProfile(name.to_string()))?;
    load_profile_from_source(source)
}

/// Parse and load a profile from BSOL source text.
pub fn load_profile_from_source(source: &str) -> Result<SchemaProfile, BsolError> {
    let document = parse_bsol_document(source).map_err(BsolError::from)?;
    load_profile_from_document(&document)
}

/// Parse and load a profile from an already-parsed document.
pub fn load_profile_from_document(document: &BsolDocument) -> Result<SchemaProfile, BsolError> {
    let profile = parse_profile_document(document)?;
    crate::compose::compose_profile(profile)
}

/// Load a profile from a filesystem path.
pub fn load_profile_from_path(path: &Path) -> Result<SchemaProfile, BsolError> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| BsolError::Import(format!("failed to read `{}`: {e}", path.display())))?;
    load_profile_from_source(&source)
}

pub fn parse_profile_document(document: &BsolDocument) -> Result<SchemaProfile, BsolError> {
    let profile_block = document
        .blocks
        .iter()
        .find(|b| b.kind == "profile")
        .ok_or_else(|| {
            BsolError::Schema("schema profile document must contain a `profile` block".into())
        })?;
    let name = profile_block
        .label
        .as_ref()
        .map(|q| q.value.clone())
        .ok_or_else(|| {
            BsolError::schema_at(
                profile_block.span,
                "`profile` block must carry a quoted label (profile name)",
            )
        })?;

    let version = assignment_string(profile_block, "version")?
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let mut imports = Vec::new();
    let mut extends = Vec::new();
    let mut migrations = Vec::new();
    let mut rules = HashMap::new();
    let mut top_level_order = Vec::new();
    for item in &profile_block.items {
        let BsolItem::Block(rule_block) = item else {
            continue;
        };
        match rule_block.kind.as_str() {
            "import_schema" => imports.push(parse_import_schema(rule_block)?),
            "extend" => extends.push(parse_extend_block(rule_block)?),
            "migration" => migrations.push(parse_migration_block(rule_block)?),
            "rule" => {
                let rule_id = rule_block
                    .label
                    .as_ref()
                    .map(|q| q.value.clone())
                    .or_else(|| assignment_string(rule_block, "id").ok().flatten())
                    .ok_or_else(|| {
                        BsolError::schema_at(
                            rule_block.span,
                            "rule block requires a quoted label or `id`",
                        )
                    })?;
                let rule = parse_block_rule(rule_block, RuleScope::TopLevel)?;
                if rule.scope == RuleScope::TopLevel {
                    top_level_order.push(rule_id.clone());
                }
                rules.insert(rule_id, rule);
            }
            other => {
                return Err(BsolError::schema_at(
                    rule_block.span,
                    format!("unexpected item `{other}` inside profile"),
                ));
            }
        }
    }

    Ok(SchemaProfile {
        name,
        version,
        rules,
        top_level_order,
        imports,
        extends,
        migrations,
    })
}

fn parse_import_schema(block: &BsolBlock) -> Result<ImportSchemaSpec, BsolError> {
    let name = block
        .label
        .as_ref()
        .map(|q| q.value.clone())
        .ok_or_else(|| {
            BsolError::schema_at(
                block.span,
                "`import_schema` block requires a quoted profile name label",
            )
        })?;
    let alias = assignment_string(block, "alias")?;
    let from = assignment_string(block, "from")?
        .ok_or_else(|| BsolError::schema_at(block.span, "`import_schema` requires `from`"))?;

    let source = if from.starts_with("@pckg/") {
        ImportSource::PckgShorthand {
            reference: from.clone(),
        }
    } else {
        match from.as_str() {
            "file" => ImportSource::File {
                path: assignment_string(block, "path")?.ok_or_else(|| {
                    BsolError::schema_at(block.span, "`from = file` requires `path`")
                })?,
            },
            "git" => ImportSource::Git {
                url: assignment_string(block, "url")?.ok_or_else(|| {
                    BsolError::schema_at(block.span, "`from = git` requires `url`")
                })?,
                rev: assignment_string(block, "rev")?.ok_or_else(|| {
                    BsolError::schema_at(block.span, "`from = git` requires `rev`")
                })?,
                path: assignment_string(block, "path")?.ok_or_else(|| {
                    BsolError::schema_at(block.span, "`from = git` requires `path`")
                })?,
            },
            "registry" => ImportSource::Registry {
                package: assignment_string(block, "package")?.ok_or_else(|| {
                    BsolError::schema_at(block.span, "`from = registry` requires `package`")
                })?,
                version: assignment_string(block, "version")?.ok_or_else(|| {
                    BsolError::schema_at(block.span, "`from = registry` requires `version`")
                })?,
                path: assignment_string(block, "path")?.ok_or_else(|| {
                    BsolError::schema_at(block.span, "`from = registry` requires `path`")
                })?,
            },
            other => {
                return Err(BsolError::schema_at(
                    block.span,
                    format!("unknown import source `{other}`"),
                ));
            }
        }
    };

    Ok(ImportSchemaSpec {
        name,
        alias,
        from: source,
        span: block.span,
    })
}

fn parse_block_rule(block: &BsolBlock, default_scope: RuleScope) -> Result<BlockRule, BsolError> {
    let id = block
        .label
        .as_ref()
        .map(|q| q.value.clone())
        .or_else(|| assignment_string(block, "id").ok().flatten())
        .unwrap_or_else(|| block.kind.clone());
    let scope = parse_scope(block)?.unwrap_or(default_scope);
    let kind_match = parse_kind_match(block)?;
    let label = parse_label(block)?;
    let cardinality = parse_cardinality(block)?;
    let allow_extra_fields = parse_bool(block, "extras").unwrap_or(false);
    let allow_extra_nested = parse_bool(block, "nested_extras").unwrap_or(false);
    let schemaless = parse_bool(block, "schemaless").unwrap_or(false);
    let extends = assignment_string(block, "extends")?;
    let mixes = assignment_list(block, "mixes").unwrap_or_default();
    let allowed_attrs = assignment_list(block, "allowed_attrs").unwrap_or_default();

    let mut fields = HashMap::new();
    let mut nested = HashMap::new();
    let mut nested_order = Vec::new();
    let mut variants = Vec::new();
    for item in &block.items {
        match item {
            BsolItem::Block(nested_block) if nested_block.kind == "field" => {
                let field_name = nested_block
                    .label
                    .as_ref()
                    .map(|q| q.value.clone())
                    .or_else(|| assignment_string(nested_block, "name").ok().flatten())
                    .ok_or_else(|| {
                        BsolError::schema_at(
                            nested_block.span,
                            "`field` block requires a quoted label or `name`",
                        )
                    })?;
                let field_rule = parse_field_rule(nested_block)?;
                fields.insert(field_name, field_rule);
            }
            BsolItem::Block(nested_block) if nested_block.kind == "nested" => {
                let nested_id = nested_block
                    .label
                    .as_ref()
                    .map(|q| q.value.clone())
                    .or_else(|| assignment_string(nested_block, "id").ok().flatten())
                    .ok_or_else(|| {
                        BsolError::schema_at(
                            nested_block.span,
                            "`nested` block requires a quoted label or `id`",
                        )
                    })?;
                let nested_rule = parse_block_rule(nested_block, RuleScope::Nested)?;
                nested_order.push(nested_id.clone());
                nested.insert(nested_id, nested_rule);
            }
            BsolItem::Block(nested_block) if nested_block.kind == "variant" => {
                variants.push(parse_variant_rule(nested_block)?);
            }
            BsolItem::Assignment(_) => {}
            BsolItem::Block(other) => {
                return Err(BsolError::schema_at(
                    other.span,
                    format!("unexpected `{}` inside rule", other.kind),
                ));
            }
        }
    }

    Ok(BlockRule {
        id,
        scope,
        kind_match,
        label,
        cardinality,
        fields,
        nested,
        nested_order,
        allow_extra_fields,
        allow_extra_nested,
        schemaless,
        extends,
        mixes,
        variants,
        allowed_attrs,
    })
}

fn parse_field_rule(block: &BsolBlock) -> Result<FieldRule, BsolError> {
    let value_type = parse_value_type(block)?;
    let required = parse_bool(block, "required").unwrap_or(false);
    let list_values = assignment_list(block, "list_values")
        .ok()
        .filter(|v| !v.is_empty());
    let allowed_attrs = assignment_list(block, "allowed_attrs").unwrap_or_default();
    let constraints = FieldConstraints {
        default_value: assignment_string(block, "default")?,
        min: assignment_string(block, "min")?.and_then(|v| v.parse().ok()),
        max: assignment_string(block, "max")?.and_then(|v| v.parse().ok()),
        pattern: assignment_string(block, "pattern")?,
        required_if: parse_if_map(block, "required_if")?,
        forbid_if: parse_if_map(block, "forbid_if")?,
    };
    Ok(FieldRule {
        value_type,
        required,
        list_values,
        constraints,
        allowed_attrs,
    })
}

fn parse_if_map(block: &BsolBlock, key: &str) -> Result<HashMap<String, String>, BsolError> {
    for item in &block.items {
        let BsolItem::Block(nested) = item else {
            continue;
        };
        if nested.kind != key {
            continue;
        }
        let mut map = HashMap::new();
        for entry in &nested.items {
            let BsolItem::Assignment(a) = entry else {
                continue;
            };
            map.insert(a.key.clone(), value_as_string(&a.value)?);
        }
        return Ok(map);
    }
    Ok(HashMap::new())
}

fn parse_variant_rule(block: &BsolBlock) -> Result<VariantRule, BsolError> {
    let name = block
        .label
        .as_ref()
        .map(|q| q.value.clone())
        .or_else(|| assignment_string(block, "name").ok().flatten())
        .ok_or_else(|| BsolError::schema_at(block.span, "`variant` block requires a name label"))?;
    Ok(VariantRule {
        name,
        require: assignment_list(block, "require").unwrap_or_default(),
        forbid: assignment_list(block, "forbid").unwrap_or_default(),
    })
}

fn parse_extend_block(block: &BsolBlock) -> Result<ExtendSpec, BsolError> {
    let base = block
        .label
        .as_ref()
        .map(|q| q.value.clone())
        .or_else(|| assignment_string(block, "base").ok().flatten())
        .ok_or_else(|| {
            BsolError::schema_at(block.span, "`extend` block requires a base profile label")
        })?;
    let mut rules = HashMap::new();
    for item in &block.items {
        let BsolItem::Block(rule_block) = item else {
            continue;
        };
        if rule_block.kind != "rule" {
            continue;
        }
        let rule_id = rule_block
            .label
            .as_ref()
            .map(|q| q.value.clone())
            .ok_or_else(|| BsolError::schema_at(rule_block.span, "extend rule requires label"))?;
        rules.insert(rule_id, parse_block_rule(rule_block, RuleScope::TopLevel)?);
    }
    Ok(ExtendSpec {
        base,
        rules,
        span: block.span,
    })
}

fn parse_migration_block(block: &BsolBlock) -> Result<MigrationSpec, BsolError> {
    let from = assignment_string(block, "from")?
        .ok_or_else(|| BsolError::schema_at(block.span, "`migration` requires `from` profile"))?;
    let mut detect = HashMap::new();
    let mut when_clauses = Vec::new();
    let mut rewrites = Vec::new();
    for item in &block.items {
        let BsolItem::Block(nested) = item else {
            continue;
        };
        match nested.kind.as_str() {
            "detect" => {
                for entry in &nested.items {
                    let BsolItem::Assignment(a) = entry else {
                        continue;
                    };
                    detect.insert(a.key.clone(), value_as_string(&a.value)?);
                }
            }
            "when" => when_clauses.push(parse_migration_when(nested)?),
            "rewrite" => rewrites.extend(parse_migration_rewrites(nested)?),
            _ => {}
        }
    }
    Ok(MigrationSpec {
        from,
        detect,
        when_clauses,
        rewrites,
        span: block.span,
    })
}

fn parse_migration_when(block: &BsolBlock) -> Result<MigrationWhenClause, BsolError> {
    Ok(MigrationWhenClause {
        block_kind: assignment_string(block, "block")?,
        field: assignment_string(block, "field")?,
        field_value: assignment_string(block, "field_value")?,
        missing_field: assignment_string(block, "missing_field")?,
    })
}

fn parse_migration_rewrites(block: &BsolBlock) -> Result<Vec<MigrationRewrite>, BsolError> {
    let mut rewrites = Vec::new();
    for item in &block.items {
        match item {
            BsolItem::Assignment(a) if a.key == "add_field" => {
                if let BsolValue::InlineMap(map) = &a.value {
                    for entry in &map.entries {
                        rewrites.push(MigrationRewrite::AddField {
                            key: entry.key.clone(),
                            value: value_as_string(&entry.value)?,
                        });
                    }
                }
            }
            BsolItem::Assignment(a) if a.key == "rename_field" => {
                if let BsolValue::InlineMap(map) = &a.value {
                    for entry in &map.entries {
                        rewrites.push(MigrationRewrite::RenameField {
                            from: entry.key.clone(),
                            to: value_as_string(&entry.value)?,
                        });
                    }
                }
            }
            BsolItem::Assignment(a) if a.key == "replace_value" => {
                let from = assignment_string_in_block(block, "from")?;
                let to = assignment_string_in_block(block, "to")?;
                if let (Some(from), Some(to)) = (from, to) {
                    rewrites.push(MigrationRewrite::ReplaceValue { from, to });
                }
            }
            BsolItem::Block(nested) if nested.kind == "replace_value" => {
                let from = assignment_string(nested, "from")?;
                let to = assignment_string(nested, "to")?;
                if let (Some(from), Some(to)) = (from, to) {
                    rewrites.push(MigrationRewrite::ReplaceValue { from, to });
                }
            }
            _ => {}
        }
    }
    Ok(rewrites)
}

fn assignment_string_in_block(block: &BsolBlock, key: &str) -> Result<Option<String>, BsolError> {
    assignment_string(block, key)
}

fn parse_value_type(block: &BsolBlock) -> Result<ValueType, BsolError> {
    let ty = assignment_string(block, "type")?
        .ok_or_else(|| BsolError::schema_at(block.span, "`field` block requires `type`"))?;
    if ty == "enum_or_quoted" {
        let values = assignment_list(block, "values")?;
        return Ok(ValueType::EnumOrQuoted(values));
    }
    parse_value_type_text(&ty).map_err(|msg| BsolError::schema_at(block.span, msg))
}

pub(crate) fn parse_value_type_text(text: &str) -> Result<ValueType, String> {
    let text = text.trim();
    if text == "quoted" {
        return Ok(ValueType::Quoted);
    }
    if text == "ident" {
        return Ok(ValueType::Ident);
    }
    if text == "u32" {
        return Ok(ValueType::U32);
    }
    if text == "i64" {
        return Ok(ValueType::I64);
    }
    if text == "f64" {
        return Ok(ValueType::F64);
    }
    if text == "bool" {
        return Ok(ValueType::Bool);
    }
    if text == "path" {
        return Ok(ValueType::Path);
    }
    if text == "list" {
        return Ok(ValueType::List);
    }
    if text == "loose" {
        return Ok(ValueType::Loose);
    }
    if text == "enum_or_quoted" {
        return Err("enum_or_quoted requires values in field block".into());
    }
    if let Some(inner) = text.strip_prefix("list[") {
        let inner = inner.strip_suffix(']').ok_or("unclosed list type")?;
        let parts = split_type_union(inner);
        let mut types = Vec::new();
        for part in parts {
            types.push(parse_value_type_text(part)?);
        }
        return Ok(ValueType::ListOf(types));
    }
    if let Some(inner) = text.strip_prefix("map[") {
        let inner = inner.strip_suffix(']').ok_or("unclosed map type")?;
        let (key, value) = inner
            .split_once(',')
            .ok_or("map type requires key, value pair")?;
        return Ok(ValueType::MapOf {
            key: Box::new(parse_value_type_text(key.trim())?),
            value: Box::new(parse_value_type_text(value.trim())?),
        });
    }
    if let Some(inner) = text.strip_prefix("ref(") {
        let rule = inner.strip_suffix(')').ok_or("unclosed ref type")?;
        return Ok(ValueType::RefTo(rule.to_string()));
    }
    if let Some(inner) = text.strip_prefix("inline(") {
        let rule = inner.strip_suffix(')').ok_or("unclosed inline type")?;
        return Ok(ValueType::Inline(rule.to_string()));
    }
    Err(format!("unknown field type `{text}`"))
}

fn split_type_union(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '[' | '(' => depth += 1,
            ']' | ')' => depth -= 1,
            '|' if depth == 0 => {
                parts.push(text[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(text[start..].trim());
    parts.retain(|p| !p.is_empty());
    parts
}

fn parse_kind_match(block: &BsolBlock) -> Result<KindMatch, BsolError> {
    let match_kind = assignment_string(block, "match")?.unwrap_or_else(|| "keyword".to_string());
    match match_kind.as_str() {
        "keyword" => {
            let keyword = assignment_string(block, "keyword")?.ok_or_else(|| {
                BsolError::schema_at(block.span, "`match = keyword` requires `keyword`")
            })?;
            Ok(KindMatch::Keyword(keyword))
        }
        "keywords" => {
            let keywords = assignment_list(block, "keywords")?;
            Ok(KindMatch::Keywords(keywords))
        }
        "free_ident" => {
            let except = assignment_list(block, "except").unwrap_or_default();
            Ok(KindMatch::FreeIdent { except })
        }
        other => Err(BsolError::schema_at(
            block.span,
            format!("unknown kind match `{other}`"),
        )),
    }
}

fn parse_scope(block: &BsolBlock) -> Result<Option<RuleScope>, BsolError> {
    let Some(scope) = assignment_string(block, "scope")? else {
        return Ok(None);
    };
    Ok(Some(match scope.as_str() {
        "top" => RuleScope::TopLevel,
        "nested" => RuleScope::Nested,
        "any" => RuleScope::Any,
        other => {
            return Err(BsolError::schema_at(
                block.span,
                format!("unknown scope `{other}`"),
            ));
        }
    }))
}

fn parse_label(block: &BsolBlock) -> Result<LabelRequirement, BsolError> {
    let Some(label) = assignment_string(block, "label")? else {
        return Ok(LabelRequirement::Optional);
    };
    Ok(match label.as_str() {
        "required" => LabelRequirement::Required,
        "forbidden" => LabelRequirement::Forbidden,
        "optional" => LabelRequirement::Optional,
        other => {
            return Err(BsolError::schema_at(
                block.span,
                format!("unknown label requirement `{other}`"),
            ));
        }
    })
}

fn parse_cardinality(block: &BsolBlock) -> Result<Cardinality, BsolError> {
    let Some(card) = assignment_string(block, "cardinality")? else {
        return Ok(Cardinality::Many);
    };
    Ok(match card.as_str() {
        "one" => Cardinality::One,
        "many" => Cardinality::Many,
        "zero_or_one" => Cardinality::ZeroOrOne,
        other => {
            return Err(BsolError::schema_at(
                block.span,
                format!("unknown cardinality `{other}`"),
            ));
        }
    })
}

fn parse_bool(block: &BsolBlock, key: &str) -> Option<bool> {
    assignment_string(block, key)
        .ok()
        .flatten()
        .map(|v| v == "true")
}

fn assignment_string(block: &BsolBlock, key: &str) -> Result<Option<String>, BsolError> {
    for item in &block.items {
        let BsolItem::Assignment(a) = item else {
            continue;
        };
        if a.key != key {
            continue;
        }
        return Ok(Some(value_as_string(&a.value)?));
    }
    Ok(None)
}

fn assignment_list(block: &BsolBlock, key: &str) -> Result<Vec<String>, BsolError> {
    for item in &block.items {
        let BsolItem::Assignment(a) = item else {
            continue;
        };
        if a.key != key {
            continue;
        }
        return value_as_list(&a.value);
    }
    Ok(Vec::new())
}

fn value_as_string(value: &BsolValue) -> Result<String, BsolError> {
    match value {
        BsolValue::QuotedString(q) => Ok(q.value.clone()),
        BsolValue::Ident(i) => Ok(i.clone()),
        BsolValue::Bool(b) => Ok(b.to_string()),
        BsolValue::Ref(r) => Ok(r.display()),
        BsolValue::BracketList(_) | BsolValue::InlineMap(_) => Err(BsolError::Schema(
            "expected string, found structured value".into(),
        )),
    }
}

fn value_as_list(value: &BsolValue) -> Result<Vec<String>, BsolError> {
    let BsolValue::BracketList(list) = value else {
        return Err(BsolError::Schema("expected list".into()));
    };
    let mut out = Vec::new();
    for item in &list.items {
        match item {
            BsolListItem::QuotedString(q) => out.push(q.value.clone()),
            BsolListItem::Ident(i) => out.push(i.clone()),
            BsolListItem::Default => out.push("default".to_string()),
            BsolListItem::Bool(b) => out.push(b.to_string()),
            BsolListItem::Ref(r) => out.push(r.display()),
            BsolListItem::InlineMap(_) | BsolListItem::InlineBlock(_) => {
                return Err(BsolError::Schema(
                    "expected flat list item, found structured value".into(),
                ));
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_project_profile() {
        let profile = load_profile("project.v1").expect("load project profile");
        assert_eq!(profile.name, "project.v1");
        assert!(profile.rule("root").is_some());
        assert!(profile.rule("target").is_some());
    }

    #[test]
    fn load_schema_meta_profile() {
        let profile = load_profile("schema.v1").expect("load schema.v1");
        assert_eq!(profile.name, "schema.v1");
        assert!(profile.rule("profile").is_some());
    }

    #[test]
    fn load_configuration_profile() {
        let profile = load_profile("configuration.v1").expect("load configuration.v1");
        assert_eq!(profile.name, "configuration.v1");
        assert!(profile.rule("config").is_some());
        assert!(profile.rule("moduleConfig").is_some());
        assert!(profile.rule("option").is_some());
    }

    #[test]
    fn all_embedded_profiles_load() {
        for (name, _) in EMBEDDED_PROFILES {
            load_profile(name).unwrap_or_else(|e| panic!("load {name}: {e}"));
        }
    }
}
