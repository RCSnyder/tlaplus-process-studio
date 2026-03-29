use once_cell::sync::Lazy;
use regex::Regex;

use crate::model::{Action, Comment, ParsedMachine};

static MODULE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^----\s+MODULE\s+([A-Za-z0-9_]+)\s+----").expect("module regex")
});
static SET_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?s)(\w+)\s*==\s*\{(.*?)\}").expect("set regex")
});
static ACTION_START_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)(?:^\(\*(.*?)\*\)\s*\r?\n)?^(\w+)\s*==[ \t]*(.*)$")
        .expect("action start regex")
});
static END_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)^====\s*$").expect("module end regex"));
static QUOTED_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"\"([^\"]+)\""#).expect("quoted regex"));
static FROM_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\b\w+State\s*=\s*\"([^\"]+)\""#).expect("from regex")
});
static FROM_IN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\b\w+State\s*\\in\s*\{([^}]+)\}"#).expect("from in regex")
});
static TO_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\b\w+State'\s*(?:=|\\in)\s*(?:\{?([^\n]+))"#).expect("to regex")
});

pub fn parse_tla(source: &str) -> ParsedMachine {
    let mut machine = ParsedMachine::empty();

    if let Some(caps) = MODULE_RE.captures(source) {
        machine.module_name = caps
            .get(1)
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "Untitled Machine".to_string());
    }

    for caps in SET_RE.captures_iter(source) {
        let set_name = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
        let body = caps.get(2).map(|m| m.as_str()).unwrap_or_default();

        if set_name.ends_with("Stages") || set_name.ends_with("States") {
            for quoted in QUOTED_RE.captures_iter(body) {
                if let Some(value) = quoted.get(1).map(|m| m.as_str().trim().to_string()) {
                    if !machine.states.contains(&value) {
                        machine.states.push(value);
                    }
                }
            }
        }
    }

    for (comment, name, body) in action_blocks(source) {
        let comment = comment.map(|text| normalize_ws(&text));

        if name.ends_with("Stages") || name.ends_with("States") {
            continue;
        }

        let mut from = FROM_RE
            .captures_iter(&body)
            .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
            .collect::<Vec<_>>();

        for caps in FROM_IN_RE.captures_iter(&body) {
            if let Some(set_body) = caps.get(1).map(|m| m.as_str()) {
                for q in QUOTED_RE.captures_iter(set_body) {
                    if let Some(val) = q.get(1).map(|m| m.as_str().to_string()) {
                        if !from.contains(&val) {
                            from.push(val);
                        }
                    }
                }
            }
        }

        let to = TO_RE
            .captures_iter(&body)
            .flat_map(|c| {
                c.get(1)
                    .map(|m| {
                        QUOTED_RE
                            .captures_iter(m.as_str())
                            .filter_map(|q| q.get(1).map(|qm| qm.as_str().to_string()))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>();

        if body.contains("=>") && from.is_empty() && to.is_empty() {
            machine
                .invariants
                .push(format!("{}: {}", name, normalize_ws(&body)));
            continue;
        }

        if from.is_empty() && to.is_empty() {
            machine.warnings.push(format!(
                "Could not infer transitions for `{}`. This is common for complex or quantified actions in MVP mode.",
                name
            ));
        }

        if let Some(ref text) = comment {
            machine.comments.push(Comment {
                target: name.clone(),
                text: text.clone(),
            });
        }

        machine.actions.push(Action {
            name,
            from,
            to,
            comment,
        });
    }

    machine.states.sort();
    machine.actions.sort_by(|a, b| a.name.cmp(&b.name));

    if machine.states.is_empty() {
        machine.warnings.push(
            "No state sets detected. Try naming sets like LeadStages or JobStates for the MVP parser."
                .to_string(),
        );
    }

    machine
}

fn normalize_ws(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn action_blocks(source: &str) -> Vec<(Option<String>, String, String)> {
    let end_index = END_RE
        .find(source)
        .map(|m| m.start())
        .unwrap_or(source.len());

    let matches = ACTION_START_RE
        .captures_iter(source)
        .filter_map(|caps| {
            let whole = caps.get(0)?;
            let body = caps.get(3)?;

            Some((
                whole.start(),
                body.start(),
                caps.get(1).map(|m| m.as_str().to_string()),
                caps.get(2)?.as_str().trim().to_string(),
            ))
        })
        .collect::<Vec<_>>();

    let mut blocks = Vec::new();

    for (index, (_, body_start, comment, name)) in matches.iter().enumerate() {
        let next_start = matches
            .get(index + 1)
            .map(|(start, _, _, _)| *start)
            .unwrap_or(end_index);
        let body_end = next_start.min(end_index);
        let body = source[*body_start..body_end].trim().to_string();

        blocks.push((comment.clone(), name.clone(), body));
    }

    blocks
}
