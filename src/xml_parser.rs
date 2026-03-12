use anyhow::{Context, Result};
use indexmap::IndexMap;
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;

/// Generic XML node representation that preserves all data for lossless roundtrip.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum XmlValue {
    Text(String),
    Node(XmlNode),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct XmlNode {
    #[serde(rename = "_tag")]
    pub tag: String,

    #[serde(rename = "_ns", skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,

    #[serde(rename = "_attrs", skip_serializing_if = "IndexMap::is_empty", default)]
    pub attrs: IndexMap<String, String>,

    #[serde(rename = "_children", skip_serializing_if = "Vec::is_empty", default)]
    pub children: Vec<XmlValue>,
}

/// Parse XML string into an XmlNode tree.
pub fn parse_xml(xml: &str) -> Result<XmlNode> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut stack: Vec<XmlNode> = Vec::new();
    let mut root: Option<XmlNode> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let node = element_to_node(e, &reader)?;
                stack.push(node);
            }
            Ok(Event::Empty(ref e)) => {
                let node = element_to_node(e, &reader)?;
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(XmlValue::Node(node));
                } else {
                    root = Some(node);
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = e
                    .unescape()
                    .context("Failed to unescape XML text")?
                    .to_string();
                if !text.is_empty() {
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(XmlValue::Text(text));
                    }
                }
            }
            Ok(Event::CData(ref e)) => {
                let text = String::from_utf8(e.to_vec()).context("Failed to decode CDATA")?;
                if !text.is_empty() {
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(XmlValue::Text(text));
                    }
                }
            }
            Ok(Event::End(_)) => {
                let node = stack.pop().context("Unexpected closing tag")?;
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(XmlValue::Node(node));
                } else {
                    root = Some(node);
                }
            }
            Ok(Event::Decl(_)) => {
                // XML declaration — we regenerate it on unpack
            }
            Ok(Event::Eof) => break,
            Ok(_) => {} // Comments, PI, etc. — skip
            Err(e) => anyhow::bail!(
                "XML parse error at position {}: {e}",
                reader.error_position()
            ),
        }
    }

    root.context("Empty XML document")
}

fn element_to_node(e: &BytesStart, reader: &Reader<&[u8]>) -> Result<XmlNode> {
    let tag = String::from_utf8(e.local_name().as_ref().to_vec()).context("Invalid tag name")?;

    let namespace = e
        .try_get_attribute("xmlns")
        .ok()
        .flatten()
        .and_then(|a| String::from_utf8(a.value.to_vec()).ok());

    let mut attrs = IndexMap::new();
    for attr in e.attributes().flatten() {
        let key = reader.decoder().decode(attr.key.as_ref())?.to_string();
        if key == "xmlns" {
            continue; // stored separately
        }
        let val = attr.unescape_value()?.to_string();
        attrs.insert(key, val);
    }

    Ok(XmlNode {
        tag,
        namespace,
        attrs,
        children: Vec::new(),
    })
}

/// Serialize XmlNode back to XML string.
pub fn to_xml(node: &XmlNode) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    write_node(&mut out, node, 0);
    out.push('\n');
    out
}

fn write_node(out: &mut String, node: &XmlNode, indent: usize) {
    let pad = "    ".repeat(indent);

    out.push_str(&pad);
    out.push('<');
    out.push_str(&node.tag);

    if let Some(ns) = &node.namespace {
        out.push_str(" xmlns=\"");
        out.push_str(ns);
        out.push('"');
    }

    for (k, v) in &node.attrs {
        out.push(' ');
        out.push_str(k);
        out.push_str("=\"");
        out.push_str(&escape_attr(v));
        out.push('"');
    }

    if node.children.is_empty() {
        out.push_str("/>");
        return;
    }

    out.push('>');

    // If single text child, inline it
    if node.children.len() == 1 {
        if let XmlValue::Text(t) = &node.children[0] {
            out.push_str(&escape_text(t));
            out.push_str("</");
            out.push_str(&node.tag);
            out.push('>');
            return;
        }
    }

    out.push('\n');
    for child in &node.children {
        match child {
            XmlValue::Text(t) => {
                out.push_str(&pad);
                out.push_str("    ");
                out.push_str(&escape_text(t));
                out.push('\n');
            }
            XmlValue::Node(n) => {
                write_node(out, n, indent + 1);
                out.push('\n');
            }
        }
    }
    out.push_str(&pad);
    out.push_str("</");
    out.push_str(&node.tag);
    out.push('>');
}

fn escape_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
