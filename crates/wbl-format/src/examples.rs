//! `[[examples]]` in a request file (FILE_FORMAT.md, D-H08).

use toml_edit::{ArrayOfTables, Item, Table, TableLike};
use wbl_core::example::{Example, Param};
use wbl_core::{Body, BodyKind, Header};

use crate::FormatError;
use crate::fields::{optional_bool, optional_integer, set, set_string_map, string, string_map};

fn kind_of(name: &str) -> Option<BodyKind> {
    match name {
        "json" => Some(BodyKind::Json),
        "xml" => Some(BodyKind::Xml),
        "text" => Some(BodyKind::Text),
        _ => None,
    }
}

pub(crate) fn kind_name(kind: BodyKind) -> &'static str {
    match kind {
        BodyKind::Json => "json",
        BodyKind::Xml => "xml",
        BodyKind::Text => "text",
    }
}

/// A `[body]` table: `kind` and `text`; in an example, `kind = "none"` removes the body.
pub(crate) fn read_body_table(
    table: &dyn TableLike,
    field: &str,
    allow_none: bool,
) -> Result<Body, FormatError> {
    let as_table = to_table(table);
    let kind = string(&as_table, "kind", &format!("{field}.kind"))?;
    if allow_none && kind == "none" {
        return Ok(Body::None);
    }
    let expected = if allow_none {
        "one of json, xml, text, none"
    } else {
        "one of json, xml, text"
    };
    let kind =
        kind_of(&kind).ok_or_else(|| FormatError::invalid(&format!("{field}.kind"), expected))?;
    Ok(Body::Text {
        kind,
        text: string(&as_table, "text", &format!("{field}.text"))?,
    })
}

/// Inline and standard tables read alike.
fn to_table(like: &dyn TableLike) -> Table {
    let mut table = Table::new();
    for (k, v) in like.iter() {
        table.insert(k, v.clone());
    }
    table
}

fn tables(item: Option<&Item>, field: &str) -> Result<Vec<Table>, FormatError> {
    let Some(item) = item else {
        return Ok(Vec::new());
    };
    if let Some(aot) = item.as_array_of_tables() {
        return Ok(aot.iter().cloned().collect());
    }
    if let Some(array) = item.as_array() {
        return array
            .iter()
            .map(|v| {
                v.as_inline_table()
                    .map(|t| to_table(t))
                    .ok_or_else(|| FormatError::invalid(field, "a list of tables"))
            })
            .collect();
    }
    Err(FormatError::invalid(field, "a list of tables"))
}

fn read_example(t: &Table) -> Result<Example, FormatError> {
    let query = tables(t.get("query"), "examples.query")?
        .iter()
        .map(|q| {
            Ok(Param {
                name: string(q, "name", "examples.query.name")?,
                value: string(q, "value", "examples.query.value")?,
                enabled: optional_bool(q, "enabled", "examples.query.enabled", true)?,
            })
        })
        .collect::<Result<_, FormatError>>()?;
    let headers = tables(t.get("headers"), "examples.headers")?
        .iter()
        .map(|h| {
            Ok(Header {
                name: string(h, "name", "examples.headers.name")?,
                value: string(h, "value", "examples.headers.value")?,
                enabled: optional_bool(h, "enabled", "examples.headers.enabled", true)?,
            })
        })
        .collect::<Result<_, FormatError>>()?;
    let body = match t.get("body").and_then(Item::as_table_like) {
        Some(b) => Some(read_body_table(b, "examples.body", true)?),
        None => None,
    };
    Ok(Example {
        id: string(t, "id", "examples.id")?,
        name: string(t, "name", "examples.name")?,
        order: optional_integer(t, "order", "examples.order")?.unwrap_or(0),
        variables: string_map(t, "variables")?,
        query,
        headers,
        body,
    })
}

pub(crate) fn read_examples(root: &Table) -> Result<Vec<Example>, FormatError> {
    tables(root.get("examples"), "examples")?
        .iter()
        .map(read_example)
        .collect()
}

/// A new table starts after a blank line, as a person would write it.
pub(crate) fn spaced(mut table: Table) -> Table {
    table.decor_mut().set_prefix("\n");
    table
}

fn named_values(items: impl Iterator<Item = (String, String, bool)>) -> Option<ArrayOfTables> {
    let mut out = ArrayOfTables::new();
    for (name, value, enabled) in items {
        let mut t = spaced(Table::new());
        set(&mut t, "name", name.as_str());
        set(&mut t, "value", value.as_str());
        if !enabled {
            set(&mut t, "enabled", false);
        }
        out.push(t);
    }
    (!out.is_empty()).then_some(out)
}

fn example_table(example: &Example) -> Table {
    let mut t = spaced(Table::new());
    set(&mut t, "id", example.id.as_str());
    set(&mut t, "name", example.name.as_str());
    set(&mut t, "order", example.order);
    set_string_map(&mut t, "variables", &example.variables);
    let query = example
        .query
        .iter()
        .map(|p| (p.name.clone(), p.value.clone(), p.enabled));
    if let Some(q) = named_values(query) {
        t.insert("query", Item::ArrayOfTables(q));
    }
    let headers = example
        .headers
        .iter()
        .map(|h| (h.name.clone(), h.value.clone(), h.enabled));
    if let Some(h) = named_values(headers) {
        t.insert("headers", Item::ArrayOfTables(h));
    }
    if let Some(body) = &example.body {
        let mut b = spaced(Table::new());
        match body {
            Body::None => set(&mut b, "kind", "none"),
            Body::Text { kind, text } => {
                set(&mut b, "kind", kind_name(*kind));
                set(&mut b, "text", text.as_str());
            }
        }
        t.insert("body", Item::Table(b));
    }
    t
}

/// Rewrites `[[examples]]` only when the examples changed.
pub(crate) fn write_examples(root: &mut Table, examples: &[Example]) {
    if read_examples(root).ok().as_deref() == Some(examples) {
        return;
    }
    if examples.is_empty() {
        root.remove("examples");
        return;
    }
    let mut aot = ArrayOfTables::new();
    for example in examples {
        aot.push(example_table(example));
    }
    root.insert("examples", Item::ArrayOfTables(aot));
}
