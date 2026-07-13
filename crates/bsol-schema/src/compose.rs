//! Profile composition: extends, mixes, extend overlays, import merge.

use std::collections::{HashMap, VecDeque};

use crate::{BlockRule, ExtendSpec, SchemaProfile};

/// Resolve `extends` and `mixes` within a single profile.
pub fn compose_profile(mut profile: SchemaProfile) -> Result<SchemaProfile, crate::BsolError> {
    validate_extends_dag(&profile.rules)?;
    let mut resolved = HashMap::new();
    for rule_id in profile.rules.keys().cloned().collect::<Vec<_>>() {
        let rule = profile.rules.remove(&rule_id).expect("rule");
        let composed = resolve_rule(rule, &profile.rules, &mut resolved)?;
        profile.rules.insert(rule_id, composed);
    }
    Ok(profile)
}

fn resolve_rule(
    mut rule: BlockRule,
    all_rules: &HashMap<String, BlockRule>,
    resolved: &mut HashMap<String, BlockRule>,
) -> Result<BlockRule, crate::BsolError> {
    if let Some(existing) = resolved.get(&rule.id) {
        return Ok(existing.clone());
    }

    if let Some(base_id) = &rule.extends {
        let base = all_rules
            .get(base_id)
            .ok_or_else(|| crate::BsolError::Schema(format!("unknown base rule `{base_id}`")))?;
        if !resolved.contains_key(base_id) {
            let base_clone = base.clone();
            let composed_base = resolve_rule(base_clone, all_rules, resolved)?;
            resolved.insert(base_id.clone(), composed_base);
        }
        let base = resolved.get(base_id).expect("base");
        rule = merge_block_rules(base, &rule);
    }

    for mix_id in &rule.mixes.clone() {
        let mix = all_rules
            .get(mix_id)
            .ok_or_else(|| crate::BsolError::Schema(format!("unknown mix rule `{mix_id}`")))?;
        if !resolved.contains_key(mix_id) {
            let mix_clone = mix.clone();
            let composed_mix = resolve_rule(mix_clone, all_rules, resolved)?;
            resolved.insert(mix_id.clone(), composed_mix);
        }
        let mix = resolved.get(mix_id).expect("mix");
        rule = merge_block_rules(mix, &rule);
    }

    let mut nested_resolved = HashMap::new();
    for (id, nested) in rule.nested.clone() {
        nested_resolved.insert(id, resolve_rule(nested, all_rules, resolved)?);
    }
    rule.nested = nested_resolved;

    resolved.insert(rule.id.clone(), rule.clone());
    Ok(rule)
}

fn merge_block_rules(base: &BlockRule, overlay: &BlockRule) -> BlockRule {
    let mut fields = base.fields.clone();
    for (key, field) in &overlay.fields {
        fields.insert(key.clone(), field.clone());
    }
    let mut nested = base.nested.clone();
    for (key, rule) in &overlay.nested {
        nested.insert(key.clone(), rule.clone());
    }
    let mut nested_order = base.nested_order.clone();
    for id in &overlay.nested_order {
        if !nested_order.iter().any(|existing| existing == id) {
            nested_order.push(id.clone());
        }
    }
    let mut variants = base.variants.clone();
    for variant in &overlay.variants {
        if !variants.iter().any(|v| v.name == variant.name) {
            variants.push(variant.clone());
        }
    }
    BlockRule {
        id: overlay.id.clone(),
        scope: overlay.scope,
        kind_match: overlay.kind_match.clone(),
        label: overlay.label,
        cardinality: overlay.cardinality,
        fields,
        nested,
        nested_order,
        allow_extra_fields: overlay.allow_extra_fields || base.allow_extra_fields,
        allow_extra_nested: overlay.allow_extra_nested || base.allow_extra_nested,
        schemaless: overlay.schemaless || base.schemaless,
        extends: overlay.extends.clone().or(base.extends.clone()),
        mixes: {
            let mut mixes = base.mixes.clone();
            for m in &overlay.mixes {
                if !mixes.iter().any(|x| x == m) {
                    mixes.push(m.clone());
                }
            }
            mixes
        },
        variants,
        allowed_attrs: {
            let mut attrs = base.allowed_attrs.clone();
            for a in &overlay.allowed_attrs {
                if !attrs.iter().any(|x| x == a) {
                    attrs.push(a.clone());
                }
            }
            attrs
        },
    }
}

/// Merge imported base profile with local overlays and extensions.
pub fn merge_profiles(base: SchemaProfile, overlay: SchemaProfile) -> SchemaProfile {
    let mut merged = base;
    merged.name = overlay.name;
    merged.version = overlay.version.max(merged.version);
    merged.migrations.extend(overlay.migrations.clone());

    for extend in &overlay.extends {
        apply_extend(&mut merged, extend);
    }

    for (id, rule) in overlay.rules {
        if let Some(existing) = merged.rules.get(&id) {
            merged.rules.insert(id, merge_block_rules(existing, &rule));
        } else {
            merged.top_level_order.push(id.clone());
            merged.rules.insert(id, rule);
        }
    }

    merged.imports.extend(overlay.imports);
    merged
}

fn apply_extend(profile: &mut SchemaProfile, extend: &ExtendSpec) {
    for (id, rule) in &extend.rules {
        if let Some(existing) = profile.rules.get(id) {
            profile
                .rules
                .insert(id.clone(), merge_block_rules(existing, rule));
        } else {
            profile.top_level_order.push(id.clone());
            profile.rules.insert(id.clone(), rule.clone());
        }
    }
}

/// Topological sort of rule extends; detects cycles.
pub fn validate_extends_dag(rules: &HashMap<String, BlockRule>) -> Result<(), crate::BsolError> {
    let mut indegree: HashMap<String, usize> = rules.keys().map(|k| (k.clone(), 0)).collect();
    let mut edges: HashMap<String, Vec<String>> = HashMap::new();
    for (id, rule) in rules {
        if let Some(base) = &rule.extends {
            if !rules.contains_key(base) {
                return Err(crate::BsolError::Schema(format!(
                    "rule `{id}` extends unknown `{base}`"
                )));
            }
            edges.entry(base.clone()).or_default().push(id.clone());
            *indegree.entry(id.clone()).or_default() += 1;
        }
    }
    let mut queue: VecDeque<String> = indegree
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(k, _)| k.clone())
        .collect();
    let mut visited = 0usize;
    while let Some(node) = queue.pop_front() {
        visited += 1;
        if let Some(children) = edges.get(&node) {
            for child in children {
                if let Some(deg) = indegree.get_mut(child) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(child.clone());
                    }
                }
            }
        }
    }
    if visited != rules.len() {
        return Err(crate::BsolError::Schema(
            "cyclic rule extends detected".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Cardinality, FieldRule, KindMatch, LabelRequirement, RuleScope, ValueType};

    fn sample_field() -> FieldRule {
        FieldRule {
            value_type: ValueType::Quoted,
            required: true,
            list_values: None,
            constraints: Default::default(),
            allowed_attrs: Vec::new(),
        }
    }

    fn sample_rule(id: &str, extends: Option<&str>) -> BlockRule {
        BlockRule {
            id: id.to_string(),
            scope: RuleScope::TopLevel,
            kind_match: KindMatch::Keyword(id.to_string()),
            label: LabelRequirement::Forbidden,
            cardinality: Cardinality::Many,
            fields: HashMap::from([(format!("{id}_field"), sample_field())]),
            nested: HashMap::new(),
            nested_order: Vec::new(),
            allow_extra_fields: false,
            allow_extra_nested: false,
            schemaless: false,
            extends: extends.map(str::to_string),
            mixes: Vec::new(),
            variants: Vec::new(),
            allowed_attrs: Vec::new(),
        }
    }

    #[test]
    fn extends_merges_fields() {
        let mut rules = HashMap::new();
        rules.insert("base".into(), sample_rule("base", None));
        rules.insert("child".into(), sample_rule("child", Some("base")));
        let profile = SchemaProfile {
            name: "test".into(),
            version: 2,
            rules,
            top_level_order: vec!["base".into(), "child".into()],
            imports: Vec::new(),
            extends: Vec::new(),
            migrations: Vec::new(),
        };
        let composed = compose_profile(profile).expect("compose");
        let child = composed.rules.get("child").expect("child");
        assert!(child.fields.contains_key("base_field"));
        assert!(child.fields.contains_key("child_field"));
    }
}
