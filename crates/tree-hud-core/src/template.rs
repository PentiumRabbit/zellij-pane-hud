pub const DEFAULT_ROW_FORMAT: &str = "{progress} {icon} {title}  {dir}";

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TemplateField {
    Progress,
    Icon,
    Title,
    Dir,
    Session,
    Tab,
    Command,
    PaneId,
}

impl TemplateField {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "progress" => Some(Self::Progress),
            "icon" => Some(Self::Icon),
            "title" => Some(Self::Title),
            "dir" => Some(Self::Dir),
            "session" => Some(Self::Session),
            "tab" => Some(Self::Tab),
            "command" => Some(Self::Command),
            "pane_id" => Some(Self::PaneId),
            _ => None,
        }
    }

    pub fn is_searchable(self) -> bool {
        !matches!(self, Self::Progress | Self::Icon)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TemplatePart {
    Literal(String),
    Field(TemplateField),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RowTemplate {
    parts: Vec<TemplatePart>,
    search_fields: Vec<TemplateField>,
}

impl RowTemplate {
    pub fn parse(input: &str) -> Self {
        if input.is_empty() {
            return Self::default();
        }
        Self::parse_non_empty(input)
    }

    pub fn parts(&self) -> &[TemplatePart] {
        &self.parts
    }

    pub fn search_fields(&self) -> &[TemplateField] {
        &self.search_fields
    }

    fn parse_non_empty(input: &str) -> Self {
        let chars = input.chars().collect::<Vec<_>>();
        let mut parts = Vec::new();
        let mut search_fields = Vec::new();
        let mut literal = String::new();
        let mut index = 0;

        while index < chars.len() {
            match chars[index] {
                '{' if chars.get(index + 1) == Some(&'{') => {
                    literal.push('{');
                    index += 2;
                }
                '{' => {
                    let Some(relative_end) = chars[index + 1..]
                        .iter()
                        .position(|character| *character == '}')
                    else {
                        literal.extend(chars[index..].iter());
                        break;
                    };
                    let end = index + 1 + relative_end;
                    let name = chars[index + 1..end].iter().collect::<String>();
                    if let Some(field) = TemplateField::from_name(&name) {
                        flush_literal(&mut parts, &mut literal);
                        parts.push(TemplatePart::Field(field));
                        if field.is_searchable() && !search_fields.contains(&field) {
                            search_fields.push(field);
                        }
                    } else {
                        literal.extend(chars[index..=end].iter());
                    }
                    index = end + 1;
                }
                '}' if chars.get(index + 1) == Some(&'}') => {
                    literal.push('}');
                    index += 2;
                }
                character => {
                    literal.push(character);
                    index += 1;
                }
            }
        }
        flush_literal(&mut parts, &mut literal);

        Self {
            parts,
            search_fields,
        }
    }
}

impl Default for RowTemplate {
    fn default() -> Self {
        Self::parse_non_empty(DEFAULT_ROW_FORMAT)
    }
}

fn flush_literal(parts: &mut Vec<TemplatePart>, literal: &mut String) {
    if literal.is_empty() {
        return;
    }
    match parts.last_mut() {
        Some(TemplatePart::Literal(existing)) => existing.push_str(literal),
        _ => parts.push(TemplatePart::Literal(std::mem::take(literal))),
    }
    literal.clear();
}
