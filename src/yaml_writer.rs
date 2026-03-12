use crate::xml_parser::{XmlNode, XmlValue};
use anyhow::Result;
use indexmap::IndexMap;
use serde_yaml::Value;

/// Convert a parsed XML tree to a compact YAML representation.
/// This is the "smart" layer that understands Salesforce metadata semantics.
pub fn xml_to_yaml(node: &XmlNode) -> Result<String> {
    let value = node_to_yaml_value(node);
    let yaml = serde_yaml::to_string(&value)?;
    Ok(yaml)
}

/// Convert YAML string back to XmlNode tree.
pub fn yaml_to_xml_node(yaml: &str) -> Result<XmlNode> {
    let value: Value = serde_yaml::from_str(yaml)?;
    let node = yaml_value_to_node(&value)?;
    Ok(node)
}

// ─── XML → YAML ───────────────────────────────────────────────

fn node_to_yaml_value(node: &XmlNode) -> Value {
    let mut map = serde_yaml::Mapping::new();

    // Always store the tag
    map.insert(
        Value::String("_tag".to_string()),
        Value::String(node.tag.clone()),
    );

    // Store namespace if present
    if let Some(ns) = &node.namespace {
        map.insert(
            Value::String("_ns".to_string()),
            Value::String(ns.clone()),
        );
    }

    // Store attributes if present
    if !node.attrs.is_empty() {
        let mut attrs_map = serde_yaml::Mapping::new();
        for (k, v) in &node.attrs {
            attrs_map.insert(
                Value::String(k.clone()),
                Value::String(v.clone()),
            );
        }
        map.insert(
            Value::String("_attrs".to_string()),
            Value::Mapping(attrs_map),
        );
    }

    // Process children — group by tag name for compactness
    if node.children.is_empty() {
        return Value::Mapping(map);
    }

    // Check if this is a simple text-only element
    if node.children.len() == 1 {
        if let XmlValue::Text(t) = &node.children[0] {
            map.insert(
                Value::String("_text".to_string()),
                Value::String(t.clone()),
            );
            return Value::Mapping(map);
        }
    }

    // Process children preserving document order.
    // Group consecutive same-tag children, but maintain order of first occurrence.
    let mut text_parts: Vec<String> = Vec::new();
    let mut groups: IndexMap<String, Vec<&XmlNode>> = IndexMap::new();

    for child in &node.children {
        match child {
            XmlValue::Text(t) => text_parts.push(t.clone()),
            XmlValue::Node(n) => {
                groups.entry(n.tag.clone()).or_default().push(n);
            }
        }
    }

    if !text_parts.is_empty() {
        if text_parts.len() == 1 {
            map.insert(
                Value::String("_text".to_string()),
                Value::String(text_parts.into_iter().next().unwrap()),
            );
        } else {
            map.insert(
                Value::String("_text".to_string()),
                Value::Sequence(text_parts.into_iter().map(Value::String).collect()),
            );
        }
    }

    // For each tag group, emit in document order of first appearance
    for (tag, nodes) in &groups {
        let all_leaves = nodes.iter().all(|n| is_text_leaf(n));
        let all_simple_kv = nodes.iter().all(|n| is_simple_kv_node(n));

        if all_leaves && nodes.len() == 1 {
            // Single text leaf: <custom>false</custom> → custom: false
            let val = parse_smart_value(leaf_text(nodes[0]));
            map.insert(Value::String(tag.clone()), val);
        } else if all_leaves {
            // Multiple text leaves → array of scalars
            let arr: Vec<Value> = nodes
                .iter()
                .map(|n| parse_smart_value(leaf_text(n)))
                .collect();
            map.insert(Value::String(tag.clone()), Value::Sequence(arr));
        } else if nodes.len() == 1 && all_simple_kv {
            // Single kv node: preserve child key order from XML
            let child_val = simple_node_to_value(nodes[0]);
            map.insert(Value::String(tag.clone()), child_val);
        } else if all_simple_kv {
            // Multiple kv nodes → array preserving each node's key order
            let arr: Vec<Value> = nodes.iter().map(|n| simple_node_to_value(n)).collect();
            map.insert(Value::String(tag.clone()), Value::Sequence(arr));
        } else if nodes.len() == 1 {
            // Single complex node — recurse
            let child_val = node_to_yaml_value(nodes[0]);
            if let Value::Mapping(mut m) = child_val {
                m.remove(&Value::String("_tag".to_string()));
                map.insert(Value::String(tag.clone()), Value::Mapping(m));
            }
        } else {
            // Multiple complex nodes → array
            let arr: Vec<Value> = nodes
                .iter()
                .map(|n| {
                    let v = node_to_yaml_value(n);
                    if let Value::Mapping(mut m) = v {
                        m.remove(&Value::String("_tag".to_string()));
                        Value::Mapping(m)
                    } else {
                        v
                    }
                })
                .collect();
            map.insert(Value::String(tag.clone()), Value::Sequence(arr));
        }
    }

    Value::Mapping(map)
}

/// Check if a node is a "kv container" — all children are named elements with text-only content.
/// e.g. <fieldPermissions><field>X</field><readable>true</readable></fieldPermissions>
/// NOT a leaf node like <custom>false</custom> (which has Text children, not Node children).
fn is_simple_kv_node(node: &XmlNode) -> bool {
    if !node.attrs.is_empty() || node.children.is_empty() {
        return false;
    }
    // Must have at least one Node child, and NO Text children
    let has_text_children = node.children.iter().any(|c| matches!(c, XmlValue::Text(_)));
    if has_text_children {
        return false;
    }
    // All child tags must be unique (otherwise it's a repeated-element container, not a kv map)
    let mut seen_tags = std::collections::HashSet::new();
    for c in &node.children {
        if let XmlValue::Node(n) = c {
            if !seen_tags.insert(&n.tag) {
                return false; // Duplicate tag — not a kv node
            }
        }
    }
    node.children.iter().all(|c| match c {
        XmlValue::Node(n) => {
            n.attrs.is_empty()
                && n.children.len() <= 1
                && n.children.iter().all(|cc| matches!(cc, XmlValue::Text(_)))
        }
        XmlValue::Text(_) => false,
    })
}

/// Check if a node is a text-only leaf (e.g. <custom>false</custom> or <target>X</target>).
fn is_text_leaf(node: &XmlNode) -> bool {
    node.attrs.is_empty()
        && node.children.len() <= 1
        && node.children.iter().all(|c| matches!(c, XmlValue::Text(_)))
}

/// Get text content from a leaf node.
fn leaf_text(node: &XmlNode) -> &str {
    node.children
        .first()
        .and_then(|c| match c {
            XmlValue::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .unwrap_or("")
}

/// Convert a simple key-value node to a compact YAML mapping.
fn simple_node_to_value(node: &XmlNode) -> Value {
    let mut map = serde_yaml::Mapping::new();

    for child in &node.children {
        match child {
            XmlValue::Text(_) => {} // Mixed text in kv node — skip (shouldn't happen in SF metadata)
            XmlValue::Node(n) => {
                let text = n
                    .children
                    .first()
                    .and_then(|c| match c {
                        XmlValue::Text(t) => Some(t.as_str()),
                        _ => None,
                    })
                    .unwrap_or("");

                // Try to parse booleans and numbers for cleaner YAML
                let val = parse_smart_value(text);
                map.insert(Value::String(n.tag.clone()), val);
            }
        }
    }

    Value::Mapping(map)
}

/// Parse text into appropriate YAML type (bool, int, float, or string).
fn parse_smart_value(text: &str) -> Value {
    match text {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        "" => Value::String(String::new()),
        _ => {
            if let Ok(n) = text.parse::<i64>() {
                Value::Number(serde_yaml::Number::from(n))
            } else if let Ok(f) = text.parse::<f64>() {
                Value::Number(serde_yaml::Number::from(f))
            } else {
                Value::String(text.to_string())
            }
        }
    }
}

// ─── YAML → XML ───────────────────────────────────────────────

fn yaml_value_to_node(value: &Value) -> Result<XmlNode> {
    let map = value
        .as_mapping()
        .ok_or_else(|| anyhow::anyhow!("Expected YAML mapping at root"))?;

    let tag = map
        .get(&Value::String("_tag".to_string()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing _tag in YAML node"))?
        .to_string();

    let namespace = map
        .get(&Value::String("_ns".to_string()))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let mut attrs = IndexMap::new();
    if let Some(Value::Mapping(a)) = map.get(&Value::String("_attrs".to_string())) {
        for (k, v) in a {
            if let (Some(key), Some(val)) = (k.as_str(), v.as_str()) {
                attrs.insert(key.to_string(), val.to_string());
            }
        }
    }

    let mut children = Vec::new();

    // Handle _text
    if let Some(text_val) = map.get(&Value::String("_text".to_string())) {
        match text_val {
            Value::String(s) => children.push(XmlValue::Text(s.clone())),
            Value::Sequence(arr) => {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        children.push(XmlValue::Text(s.to_string()));
                    }
                }
            }
            _ => {
                children.push(XmlValue::Text(yaml_value_to_string(text_val)));
            }
        }
    }

    // Process all other keys as child elements
    let reserved = ["_tag", "_ns", "_attrs", "_text"];
    for (key, val) in map {
        let key_str = match key.as_str() {
            Some(s) => s,
            None => continue,
        };
        if reserved.contains(&key_str) {
            continue;
        }

        match val {
            Value::Mapping(m) => {
                // Could be a single complex node or a single simple kv node
                let child = reconstruct_child_node(key_str, &Value::Mapping(m.clone()))?;
                children.push(XmlValue::Node(child));
            }
            Value::Sequence(arr) => {
                // Array of nodes with same tag
                for item in arr {
                    let child = reconstruct_child_node(key_str, item)?;
                    children.push(XmlValue::Node(child));
                }
            }
            _ => {
                // Simple scalar child element
                let child = XmlNode {
                    tag: key_str.to_string(),
                    namespace: None,
                    attrs: IndexMap::new(),
                    children: vec![XmlValue::Text(yaml_value_to_string(val))],
                };
                children.push(XmlValue::Node(child));
            }
        }
    }

    Ok(XmlNode {
        tag,
        namespace,
        attrs,
        children,
    })
}

fn reconstruct_child_node(tag: &str, value: &Value) -> Result<XmlNode> {
    match value {
        Value::Mapping(m) => {
            // Check if this mapping has meta keys (complex node) or just kv pairs (simple node)
            let has_meta_keys = m.contains_key(&Value::String("_ns".to_string()))
                || m.contains_key(&Value::String("_attrs".to_string()))
                || m.contains_key(&Value::String("_text".to_string()));

            // Also check if any values are themselves mappings/sequences — that means complex children
            let has_complex_values = m.iter().any(|(_, v)| {
                matches!(v, Value::Mapping(_) | Value::Sequence(_))
            });

            if has_meta_keys || has_complex_values {
                // Complex node — add _tag back and recurse
                let mut full_map = serde_yaml::Mapping::new();
                full_map.insert(
                    Value::String("_tag".to_string()),
                    Value::String(tag.to_string()),
                );
                for (k, v) in m {
                    full_map.insert(k.clone(), v.clone());
                }
                yaml_value_to_node(&Value::Mapping(full_map))
            } else {
                // Simple kv node — each entry becomes a child element with text
                let mut children = Vec::new();
                for (k, v) in m {
                    if let Some(key) = k.as_str() {
                        let text = yaml_value_to_string(v);
                        let child_node = XmlNode {
                            tag: key.to_string(),
                            namespace: None,
                            attrs: IndexMap::new(),
                            children: if text.is_empty() {
                                Vec::new()
                            } else {
                                vec![XmlValue::Text(text)]
                            },
                        };
                        children.push(XmlValue::Node(child_node));
                    }
                }
                Ok(XmlNode {
                    tag: tag.to_string(),
                    namespace: None,
                    attrs: IndexMap::new(),
                    children,
                })
            }
        }
        _ => {
            // Scalar value — simple text element
            Ok(XmlNode {
                tag: tag.to_string(),
                namespace: None,
                attrs: IndexMap::new(),
                children: vec![XmlValue::Text(yaml_value_to_string(value))],
            })
        }
    }
}

fn yaml_value_to_string(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Null => String::new(),
        _ => serde_yaml::to_string(val).unwrap_or_default().trim().to_string(),
    }
}
