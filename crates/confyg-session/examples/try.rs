//! A REPL over the real session, so a human can drive v0.1 before the web host exists. Not part
//! of the product — no tests, no polish — but it is the repro vehicle for the two defects in
//! `docs/debug/2026-09-04-phase-a-hands-on-findings.md`, and Phase B's renderer walks this same
//! IR.
//!
//!     cargo run -p confyg-session --example try -- \
//!       crates/confyg-session/examples/demo.schema.json \
//!       crates/confyg-session/examples/demo.toml
//!
//! The fixtures next to this file carry an out-of-range value, a key the Schema never heard of,
//! an optional section, a bounded array and comments to preserve. Any Schema works.
//!
//! Commands: `set <path> <json>` · `unset <path>` · `add <path>` · `rm <path> <i>`
//!           `on|off <path>` · `undo` · `redo` · `as toml|json|yaml` · `text` · `q`
//! Paths are dotted, with `[i]` for array indices: `servers[0].host`.

use confy_core::model::document::DocFormat;
use confy_core::model::node::{Path, Seg};
use confy_core::schema::types::SchemaSource;
use confyg_form::ir::{FormNode, Occupancy, Presence};
use confyg_session::lower::SetterIntent;
use confyg_session::session::{Request, Session, SessionCommand, SetterSnapshot};
use std::io::{self, Write};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(schema_path) = args.first() else {
        eprintln!("usage: try <schema.json> [config file]");
        return;
    };
    let schema_text = std::fs::read_to_string(schema_path).expect("read schema");

    let mut s = Session::new();
    s.dispatch(Request::Command(SessionCommand::LoadSchema {
        source: SchemaSource::Local(schema_path.clone()),
        text: schema_text,
    }));

    let mut snap = if let Some(cfg) = args.get(1) {
        let text = std::fs::read_to_string(cfg).expect("read config");
        let fmt = match cfg.rsplit('.').next() {
            Some("json") | Some("jsonc") => DocFormat::Json,
            Some("yaml") | Some("yml") => DocFormat::Yaml,
            _ => DocFormat::Toml,
        };
        s.dispatch(Request::Command(SessionCommand::Open {
            text,
            fmt,
            path: Some(cfg.clone()),
        }))
    } else {
        // No file: an empty document of the Schema's own shape, so every field reads Absent.
        s.dispatch(Request::Command(SessionCommand::Open {
            text: String::new(),
            fmt: DocFormat::Toml,
            path: None,
        }))
    };

    show(&snap);
    loop {
        print!("\n> ");
        io::stdout().flush().ok();
        let mut line = String::new();
        if io::stdin().read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        let line = line.trim();
        let (cmd, rest) = line.split_once(' ').unwrap_or((line, ""));
        let rest = rest.trim();
        let req = match cmd {
            "q" | "quit" => return,
            "text" => {
                println!("{}", snap.text);
                continue;
            }
            "set" => {
                let Some((p, v)) = rest.split_once(' ') else {
                    println!("set <path> <json>");
                    continue;
                };
                match serde_json::from_str(v.trim()) {
                    Ok(value) => Request::Intent(SetterIntent::SetValue {
                        path: parse_path(p),
                        value,
                    }),
                    Err(e) => {
                        println!("that is not JSON: {e}");
                        continue;
                    }
                }
            }
            "unset" => Request::Intent(SetterIntent::Unset {
                path: parse_path(rest),
            }),
            "add" => Request::Intent(SetterIntent::AddRepeatItem {
                path: parse_path(rest),
            }),
            "rm" => {
                let Some((p, i)) = rest.rsplit_once(' ') else {
                    println!("rm <path> <index>");
                    continue;
                };
                Request::Intent(SetterIntent::RemoveRepeatItem {
                    path: parse_path(p),
                    index: i.trim().parse().unwrap_or(0),
                })
            }
            "on" | "off" => Request::Intent(SetterIntent::ToggleGroup {
                path: parse_path(rest),
                enable: cmd == "on",
            }),
            "undo" => Request::Command(SessionCommand::Undo),
            "redo" => Request::Command(SessionCommand::Redo),
            "as" => Request::Command(SessionCommand::ConvertFormat(match rest {
                "json" => DocFormat::Json,
                "yaml" => DocFormat::Yaml,
                _ => DocFormat::Toml,
            })),
            "" => continue,
            other => {
                println!("unknown command {other:?}");
                continue;
            }
        };
        snap = s.dispatch(req);
        show(&snap);
    }
}

fn parse_path(text: &str) -> Path {
    let mut path = Path::new();
    for part in text.split('.').filter(|p| !p.is_empty()) {
        let (key, rest) = part.split_once('[').unwrap_or((part, ""));
        if !key.is_empty() {
            path.push(Seg::Key(key.to_owned()));
        }
        for idx in rest.split('[') {
            if let Some(n) = idx.trim_end_matches(']').parse().ok() {
                path.push(Seg::Index(n));
            }
        }
    }
    path
}

fn show(snap: &SetterSnapshot) {
    println!("\n--- form ---");
    outline(&snap.ir, 0);
    if !snap.summary.items.is_empty() {
        println!("\n--- what is wrong ---");
        for i in &snap.summary.items {
            println!("  {} — {} ({})", i.title, i.message, i.keyword);
        }
    }
    for n in &snap.notices {
        println!("  notice [{}] {}", n.code, n.message);
    }
    if let Some(f) = &snap.fetch {
        println!("  the host would fetch: {:?}", f.source);
    }
    println!("\n--- bytes ---\n{}", snap.text);
    println!("(undo {} / redo {})", snap.can_undo, snap.can_redo);
}

fn outline(node: &FormNode, depth: usize) {
    let pad = "  ".repeat(depth);
    match node {
        FormNode::Field {
            path,
            widget,
            intended,
            presence,
            meta,
        } => {
            let clamp = if widget == intended {
                String::new()
            } else {
                format!(" (wanted {intended:?})")
            };
            let state = match presence {
                Presence::Set { literal } => format!("= {literal}"),
                Presence::Absent {
                    default: Some(d), ..
                } => format!("unset, default {d}"),
                Presence::Absent { .. } => "unset".to_owned(),
                Presence::Invalid {
                    literal,
                    violations,
                } => {
                    format!("= {literal}  <-- {}", violations.len().max(1))
                }
            };
            let req = if meta.required { " *" } else { "" };
            let unit = meta
                .unit
                .as_deref()
                .map(|u| format!(" {u}"))
                .unwrap_or_default();
            println!("{pad}{}{req}: {widget:?}{clamp}  {state}{unit}", name(path));
        }
        FormNode::Group {
            path,
            children,
            occupancy,
            toggle,
            ..
        } => {
            let t = toggle.as_ref().map(|_| " [toggleable]").unwrap_or("");
            println!("{pad}{} {{{}}}{t}", name(path), occ(*occupancy));
            for c in children {
                outline(c, depth + 1);
            }
        }
        FormNode::Repeat {
            path,
            items,
            occupancy,
            bounds,
            ..
        } => {
            println!(
                "{pad}{} [{} items, {:?}..{:?}] {{{}}}",
                name(path),
                items.len(),
                bounds.min,
                bounds.max,
                occ(*occupancy)
            );
            for c in items {
                outline(c, depth + 1);
            }
        }
        FormNode::Unknown { path, raw_preview } => {
            println!("{pad}{}: not in the schema — {raw_preview}", name(path));
        }
        FormNode::Cyclic { path, schema_ptr } => {
            println!("{pad}{}: cyclic {schema_ptr}", name(path));
        }
    }
}

fn occ(o: Occupancy) -> &'static str {
    match o {
        Occupancy::Absent => "absent",
        Occupancy::Empty => "empty",
        Occupancy::Populated => "populated",
    }
}

fn name(path: &Path) -> String {
    if path.is_empty() {
        return "<root>".to_owned();
    }
    match path.last() {
        Some(Seg::Key(k)) => k.clone(),
        Some(Seg::Index(i)) => format!("[{i}]"),
        None => String::new(),
    }
}
