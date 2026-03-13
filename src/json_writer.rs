use crate::constants;
use crate::xml_parser::{XmlNode, XmlValue};
use anyhow::Result;
use indexmap::IndexMap;
use serde_json::Value;

/// Convert a parsed XML tree to compact single-line JSON.
/// Uses an order-preserving array-based children approach: every child element
/// is stored inside `_children` as `{"_tag":"...","key":"val",...}` objects,
/// which guarantees document-order fidelity (crucial for order-sensitive types
/// like Flow and FlexiPage).
pub fn xml_to_json(node: &XmlNode) -> Result<String> {
    let value = node_to_json_value(node);
    let json = serde_json::to_string(&value)?;
    Ok(json)
}

/// Parse compact JSON back into an XmlNode tree.
pub fn json_to_xml_node(json: &str) -> Result<XmlNode> {
    let value: Value = serde_json::from_str(json)?;
    json_value_to_node(&value)
}

// ─── XML → JSON ───────────────────────────────────────────────

fn node_to_json_value(node: &XmlNode) -> Value {
    let mut map = serde_json::Map::new();

    map.insert(
        constants::KEY_TAG.to_string(),
        Value::String(node.tag.clone()),
    );

    if let Some(ns) = &node.namespace {
        map.insert(constants::KEY_NS.to_string(), Value::String(ns.clone()));
    }

    if !node.attrs.is_empty() {
        let mut attrs = serde_json::Map::new();
        for (k, v) in &node.attrs {
            attrs.insert(k.clone(), Value::String(v.clone()));
        }
        map.insert(constants::KEY_ATTRS.to_string(), Value::Object(attrs));
    }

    if node.children.is_empty() {
        return Value::Object(map);
    }

    // Single text child
    if node.children.len() == 1 {
        if let XmlValue::Text(t) = &node.children[0] {
            map.insert(constants::KEY_TEXT.to_string(), smart_json_value(t));
            return Value::Object(map);
        }
    }

    // Multiple text children (rare)
    let text_parts: Vec<&str> = node
        .children
        .iter()
        .filter_map(|c| match c {
            XmlValue::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();

    if !text_parts.is_empty() {
        if text_parts.len() == 1 {
            map.insert(
                constants::KEY_TEXT.to_string(),
                Value::String(text_parts[0].to_string()),
            );
        } else {
            map.insert(
                constants::KEY_TEXT.to_string(),
                Value::Array(
                    text_parts
                        .iter()
                        .map(|t| Value::String(t.to_string()))
                        .collect(),
                ),
            );
        }
    }

    // All element children go into _children array, preserving exact document order
    let child_nodes: Vec<Value> = node
        .children
        .iter()
        .filter_map(|c| match c {
            XmlValue::Node(n) => Some(child_node_to_json(n)),
            _ => None,
        })
        .collect();

    if !child_nodes.is_empty() {
        map.insert(
            constants::KEY_CHILDREN.to_string(),
            Value::Array(child_nodes),
        );
    }

    Value::Object(map)
}

/// Convert a child node to a compact JSON object.
/// For simple text-leaf children, produce `{"_tag":"name","_text":"value"}`.
/// For kv-container children (all children are text-leaves with unique tags),
/// produce `{"_tag":"name","field":"val",...}`.
/// For complex children, recurse fully.
fn child_node_to_json(node: &XmlNode) -> Value {
    // Text leaf: <foo>bar</foo>
    if is_text_leaf(node) {
        let mut m = serde_json::Map::new();
        m.insert(
            constants::KEY_TAG.to_string(),
            Value::String(node.tag.clone()),
        );
        let text = leaf_text(node);
        m.insert(constants::KEY_TEXT.to_string(), smart_json_value(text));
        return Value::Object(m);
    }

    // Simple kv container: all children are text-leaves with unique tags
    if is_simple_kv_node(node) {
        let mut m = serde_json::Map::new();
        m.insert(
            constants::KEY_TAG.to_string(),
            Value::String(node.tag.clone()),
        );
        for child in &node.children {
            if let XmlValue::Node(n) = child {
                m.insert(n.tag.clone(), smart_json_value(leaf_text(n)));
            }
        }
        return Value::Object(m);
    }

    // Complex node: full recursion
    node_to_json_value(node)
}

fn smart_json_value(text: &str) -> Value {
    match text {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        _ => Value::String(text.to_string()),
    }
}

fn is_text_leaf(node: &XmlNode) -> bool {
    node.attrs.is_empty()
        && node.children.len() <= 1
        && node.children.iter().all(|c| matches!(c, XmlValue::Text(_)))
}

fn leaf_text(node: &XmlNode) -> &str {
    node.children
        .first()
        .and_then(|c| match c {
            XmlValue::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .unwrap_or("")
}

fn is_simple_kv_node(node: &XmlNode) -> bool {
    if !node.attrs.is_empty() || node.children.is_empty() {
        return false;
    }
    let has_text_children = node.children.iter().any(|c| matches!(c, XmlValue::Text(_)));
    if has_text_children {
        return false;
    }
    let mut seen_tags = std::collections::HashSet::new();
    for c in &node.children {
        if let XmlValue::Node(n) = c {
            if !seen_tags.insert(&n.tag) {
                return false;
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

// ─── JSON → XML ───────────────────────────────────────────────

fn json_value_to_node(value: &Value) -> Result<XmlNode> {
    let obj = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Expected JSON object at root"))?;

    let tag = obj
        .get(constants::KEY_TAG)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing _tag in JSON node"))?
        .to_string();

    let namespace = obj
        .get(constants::KEY_NS)
        .and_then(|v| v.as_str())
        .map(String::from);

    let mut attrs = IndexMap::new();
    if let Some(Value::Object(a)) = obj.get(constants::KEY_ATTRS) {
        for (k, v) in a {
            if let Some(val) = v.as_str() {
                attrs.insert(k.clone(), val.to_string());
            }
        }
    }

    let mut children = Vec::new();

    // Handle _text
    if let Some(text_val) = obj.get(constants::KEY_TEXT) {
        match text_val {
            Value::String(s) => children.push(XmlValue::Text(s.clone())),
            Value::Bool(b) => children.push(XmlValue::Text(b.to_string())),
            Value::Number(n) => children.push(XmlValue::Text(n.to_string())),
            Value::Array(arr) => {
                for item in arr {
                    children.push(XmlValue::Text(json_value_to_string(item)));
                }
            }
            Value::Null => {}
            _ => children.push(XmlValue::Text(json_value_to_string(text_val))),
        }
    }

    // Handle _children array (order-preserving)
    if let Some(Value::Array(arr)) = obj.get(constants::KEY_CHILDREN) {
        for item in arr {
            let child = reconstruct_child_from_json(item)?;
            children.push(XmlValue::Node(child));
        }
    }

    // Also handle non-reserved keys (for backward compat / alternative format)
    let reserved = [
        constants::KEY_TAG,
        constants::KEY_NS,
        constants::KEY_ATTRS,
        constants::KEY_TEXT,
        constants::KEY_CHILDREN,
    ];
    for (key, val) in obj {
        if reserved.contains(&key.as_str()) {
            continue;
        }
        match val {
            Value::Object(_) => {
                let child = reconstruct_named_child_from_json(key, val)?;
                children.push(XmlValue::Node(child));
            }
            Value::Array(arr) => {
                for item in arr {
                    let child = reconstruct_named_child_from_json(key, item)?;
                    children.push(XmlValue::Node(child));
                }
            }
            _ => {
                let child = XmlNode {
                    tag: key.clone(),
                    namespace: None,
                    attrs: IndexMap::new(),
                    children: vec![XmlValue::Text(json_value_to_string(val))],
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

fn reconstruct_child_from_json(value: &Value) -> Result<XmlNode> {
    let obj = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Expected JSON object in _children array"))?;

    let tag = obj
        .get(constants::KEY_TAG)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing _tag in _children element"))?
        .to_string();

    // Check if this has _children or _text (recursive structure)
    let has_children_arr = obj.contains_key(constants::KEY_CHILDREN);
    let has_ns = obj.contains_key(constants::KEY_NS);
    let has_attrs = obj.contains_key(constants::KEY_ATTRS);

    if has_children_arr || has_ns || has_attrs {
        // Full recursive node
        return json_value_to_node(value);
    }

    // Check for _text (text leaf)
    if let Some(text_val) = obj.get(constants::KEY_TEXT) {
        let text = json_value_to_string(text_val);
        return Ok(XmlNode {
            tag,
            namespace: None,
            attrs: IndexMap::new(),
            children: if text.is_empty() {
                Vec::new()
            } else {
                vec![XmlValue::Text(text)]
            },
        });
    }

    // Otherwise it's a kv container: keys besides _tag are child elements
    let mut children = Vec::new();
    for (k, v) in obj {
        if k == constants::KEY_TAG {
            continue;
        }
        let text = json_value_to_string(v);
        children.push(XmlValue::Node(XmlNode {
            tag: k.clone(),
            namespace: None,
            attrs: IndexMap::new(),
            children: if text.is_empty() {
                Vec::new()
            } else {
                vec![XmlValue::Text(text)]
            },
        }));
    }

    Ok(XmlNode {
        tag,
        namespace: None,
        attrs: IndexMap::new(),
        children,
    })
}

fn reconstruct_named_child_from_json(tag: &str, value: &Value) -> Result<XmlNode> {
    match value {
        Value::Object(m) => {
            if m.contains_key(constants::KEY_TAG) {
                // Has its own tag — just parse as a full node
                json_value_to_node(value)
            } else {
                // kv node without _tag
                let mut children = Vec::new();
                for (k, v) in m {
                    let text = json_value_to_string(v);
                    children.push(XmlValue::Node(XmlNode {
                        tag: k.clone(),
                        namespace: None,
                        attrs: IndexMap::new(),
                        children: if text.is_empty() {
                            Vec::new()
                        } else {
                            vec![XmlValue::Text(text)]
                        },
                    }));
                }
                Ok(XmlNode {
                    tag: tag.to_string(),
                    namespace: None,
                    attrs: IndexMap::new(),
                    children,
                })
            }
        }
        _ => Ok(XmlNode {
            tag: tag.to_string(),
            namespace: None,
            attrs: IndexMap::new(),
            children: vec![XmlValue::Text(json_value_to_string(value))],
        }),
    }
}

fn json_value_to_string(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Null => String::new(),
        _ => val.to_string(),
    }
}
