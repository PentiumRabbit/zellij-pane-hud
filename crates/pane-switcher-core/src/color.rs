use kdl::{KdlDocument, KdlNode};

use crate::style::ForegroundColor;
use crate::template::{RowTemplate, TemplatePart};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProgressColors {
    pub running: ForegroundColor,
    pub idle: ForegroundColor,
    pub exited: ForegroundColor,
    pub unknown: ForegroundColor,
}

impl Default for ProgressColors {
    fn default() -> Self {
        Self {
            running: ForegroundColor::Cyan,
            idle: ForegroundColor::Green,
            exited: ForegroundColor::Red,
            unknown: ForegroundColor::DarkGray,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ColorConfig {
    part_colors: Vec<Option<ForegroundColor>>,
    pub progress: ProgressColors,
}

impl ColorConfig {
    pub fn parse(value: &str, row_format: &RowTemplate) -> Self {
        let Ok(document) = value.parse::<KdlDocument>() else {
            return Self::default();
        };
        let mut progress = ProgressColors::default();
        let mut rules = Vec::new();

        for (order, node) in document.nodes().iter().enumerate() {
            let name = node.name().value();
            if name == "progress" {
                if let Some(children) = node.children() {
                    for child in children.nodes() {
                        let Some(color) = node_color(child) else {
                            continue;
                        };
                        match child.name().value() {
                            "running" => progress.running = color,
                            "idle" => progress.idle = color,
                            "exited" => progress.exited = color,
                            "unknown" => progress.unknown = color,
                            _ => {}
                        }
                    }
                }
                continue;
            }
            let Some(color) = node_color(node) else {
                continue;
            };
            let template = RowTemplate::parse(name);
            if template
                .parts()
                .iter()
                .all(|part| matches!(part, TemplatePart::Literal(_)))
            {
                continue;
            }
            rules.push((order, template, color));
        }

        Self {
            part_colors: compile_part_colors(row_format, &rules),
            progress,
        }
    }

    pub fn part_color(&self, index: usize) -> Option<ForegroundColor> {
        self.part_colors.get(index).copied().flatten()
    }
}

fn node_color(node: &KdlNode) -> Option<ForegroundColor> {
    let value = node.entries().first()?.value().as_string()?;
    parse_color(value)
}

fn parse_color(value: &str) -> Option<ForegroundColor> {
    match value.trim().to_ascii_lowercase().replace('_', "-").as_str() {
        "default" => Some(ForegroundColor::Default),
        "gray" | "grey" => Some(ForegroundColor::Gray),
        "green" => Some(ForegroundColor::Green),
        "cyan" => Some(ForegroundColor::Cyan),
        "magenta" => Some(ForegroundColor::Magenta),
        "orange" => Some(ForegroundColor::Orange),
        "blue" => Some(ForegroundColor::Blue),
        "red" => Some(ForegroundColor::Red),
        "yellow" => Some(ForegroundColor::Yellow),
        "dark-gray" | "dark-grey" => Some(ForegroundColor::DarkGray),
        "border" => Some(ForegroundColor::Border),
        _ => None,
    }
}

fn compile_part_colors(
    row_format: &RowTemplate,
    rules: &[(usize, RowTemplate, ForegroundColor)],
) -> Vec<Option<ForegroundColor>> {
    let row_parts = row_format.parts();
    let mut part_colors = vec![None; row_parts.len()];
    let mut matches = Vec::new();

    for (order, rule, color) in rules {
        let rule_parts = rule.parts();
        if rule_parts.is_empty() || rule_parts.len() > row_parts.len() {
            continue;
        }
        for start in 0..=row_parts.len() - rule_parts.len() {
            let end = start + rule_parts.len();
            if row_parts[start..end] == *rule_parts {
                matches.push((rule_parts.len(), *order, start, end, *color));
            }
        }
    }

    matches.sort_by_key(|(length, order, _, _, _)| (*length, *order));
    for (_, _, start, end, color) in matches {
        for part_color in &mut part_colors[start..end] {
            *part_color = Some(color);
        }
    }
    part_colors
}
