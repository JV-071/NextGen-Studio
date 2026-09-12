use crate::document::{Document, MAX_BYTES};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

const MAX_STYLE_FILES: usize = 512;
const MAX_STYLE_DEPTH: usize = 64;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VisualState {
    pub hovered: bool,
    pub pressed: bool,
    pub disabled: bool,
    pub checked: bool,
    pub on: bool,
    pub focused: bool,
    pub dragging: bool,
}

#[derive(Clone, Debug, Default)]
struct StateRule {
    conditions: Vec<(String, bool)>,
    properties: HashMap<String, String>,
}

#[derive(Clone, Debug, Default)]
struct Style {
    base: Option<String>,
    properties: HashMap<String, String>,
    states: Vec<StateRule>,
}

#[derive(Clone, Debug, Default)]
pub struct ResolvedStyle {
    pub properties: HashMap<String, String>,
}

impl ResolvedStyle {
    pub fn get(&self, key: &str) -> &str {
        self.properties.get(key).map_or("", String::as_str)
    }
}

#[derive(Clone, Debug, Default)]
pub struct StyleBook {
    variables: HashMap<String, String>,
    styles: HashMap<String, Style>,
    pub files: usize,
    pub warnings: Vec<String>,
}

impl StyleBook {
    pub fn load(root: &Path) -> Self {
        let mut book = Self::default();
        let style_root = root.join("data").join("styles");
        let mut paths = Vec::new();
        collect_files(&style_root, "otui", &mut paths, MAX_STYLE_FILES);
        paths.sort();
        for path in paths {
            match fs::metadata(&path) {
                Ok(meta) if meta.len() as usize <= MAX_BYTES => match fs::read_to_string(&path) {
                    Ok(text) => {
                        book.parse(&text);
                        book.files += 1;
                    }
                    Err(error) => book.warnings.push(format!("{}: {error}", path.display())),
                },
                Ok(_) => book
                    .warnings
                    .push(format!("{} excede o limite de leitura", path.display())),
                Err(error) => book.warnings.push(format!("{}: {error}", path.display())),
            }
        }
        book
    }

    pub fn is_empty(&self) -> bool {
        self.styles.is_empty()
    }

    pub fn style_count(&self) -> usize {
        self.styles.len()
    }

    fn parse(&mut self, text: &str) {
        let lines: Vec<&str> = text.lines().collect();
        let mut index = 0;
        while index < lines.len() {
            let raw = lines[index];
            let trimmed = raw.trim();
            let indent = leading_spaces(raw);
            if indent == 0 && trimmed.starts_with('&') {
                if let Some((name, value)) = trimmed.split_once(':') {
                    self.variables
                        .insert(name.trim().to_owned(), value.trim().to_owned());
                }
                index += 1;
                continue;
            }
            if indent != 0 || trimmed.is_empty() || trimmed.starts_with("//") {
                index += 1;
                continue;
            }
            let Some((name, base)) = trimmed.split_once('<') else {
                index += 1;
                continue;
            };
            let name = name.trim();
            if name.is_empty() {
                index += 1;
                continue;
            }
            let mut style = Style {
                base: Some(base.trim().to_owned()).filter(|value| !value.is_empty()),
                ..Default::default()
            };
            index += 1;
            while index < lines.len()
                && (lines[index].trim().is_empty() || leading_spaces(lines[index]) > 0)
            {
                let line = lines[index];
                let value = line.trim();
                let level = leading_spaces(line);
                if level == 2 && value.starts_with('$') && value.ends_with(':') {
                    let mut rule = StateRule::default();
                    rule.conditions = value[..value.len() - 1]
                        .split_whitespace()
                        .map(|condition| {
                            let enabled = !condition.starts_with('!');
                            (
                                condition
                                    .trim_start_matches('$')
                                    .trim_start_matches('!')
                                    .to_ascii_lowercase(),
                                enabled,
                            )
                        })
                        .collect();
                    index += 1;
                    while index < lines.len() && leading_spaces(lines[index]) > 2 {
                        if leading_spaces(lines[index]) == 4 {
                            if let Some((key, value)) = lines[index].trim().split_once(':') {
                                rule.properties
                                    .insert(key.trim().to_owned(), self.expand(value.trim()));
                            }
                        }
                        index += 1;
                    }
                    style.states.push(rule);
                    continue;
                }
                if level == 2 {
                    if let Some((key, value)) = value.split_once(':') {
                        style
                            .properties
                            .insert(key.trim().to_owned(), self.expand(value.trim()));
                    }
                }
                index += 1;
            }
            self.styles.insert(name.to_owned(), style);
        }
        let variables = self.variables.clone();
        for style in self.styles.values_mut() {
            for value in style.properties.values_mut() {
                *value = expand_with(&variables, value);
            }
            for state in &mut style.states {
                for value in state.properties.values_mut() {
                    *value = expand_with(&variables, value);
                }
            }
        }
    }

    fn expand(&self, value: &str) -> String {
        expand_with(&self.variables, value)
    }

    pub fn resolve(&self, document: &Document, node: usize, state: VisualState) -> ResolvedStyle {
        let Some(widget) = document.nodes.get(node) else {
            return ResolvedStyle::default();
        };
        let kind = widget.name.split('<').next().unwrap_or(&widget.name).trim();
        let mut properties = HashMap::new();
        let mut visiting = HashSet::new();
        self.resolve_document_named(document, kind, state, &mut properties, &mut visiting, 0);
        for property in &widget.properties {
            if !property.block && !property.key.starts_with('@') {
                properties.insert(property.key.clone(), self.expand(&property.value));
            }
        }
        for child in document
            .nodes
            .iter()
            .filter(|candidate| candidate.parent == Some(node) && !candidate.widget)
        {
            if state_matches(&parse_conditions(&child.name), state) {
                for property in &child.properties {
                    if !property.block {
                        properties.insert(property.key.clone(), self.expand(&property.value));
                    }
                }
            }
        }
        ResolvedStyle { properties }
    }

    fn resolve_document_named(
        &self,
        document: &Document,
        name: &str,
        state: VisualState,
        out: &mut HashMap<String, String>,
        visiting: &mut HashSet<String>,
        depth: usize,
    ) {
        if depth >= MAX_STYLE_DEPTH || !visiting.insert(format!("document:{name}")) {
            return;
        }
        if let Some((index, definition)) = document.nodes.iter().enumerate().find(|(_, node)| {
            node.parent.is_none()
                && node
                    .name
                    .split_once('<')
                    .is_some_and(|(style, _)| style.trim() == name)
        }) {
            let base = definition
                .name
                .split_once('<')
                .map(|(_, base)| base.trim())
                .unwrap_or("");
            if !base.is_empty() {
                self.resolve_document_named(document, base, state, out, visiting, depth + 1);
            }
            for property in &definition.properties {
                if !property.block {
                    out.insert(property.key.clone(), self.expand(&property.value));
                }
            }
            for child in document
                .nodes
                .iter()
                .filter(|candidate| candidate.parent == Some(index) && !candidate.widget)
            {
                if state_matches(&parse_conditions(&child.name), state) {
                    for property in &child.properties {
                        if !property.block {
                            out.insert(property.key.clone(), self.expand(&property.value));
                        }
                    }
                }
            }
        } else {
            self.resolve_named(name, state, out, visiting, depth);
        }
        visiting.remove(&format!("document:{name}"));
    }

    fn resolve_named(
        &self,
        name: &str,
        state: VisualState,
        out: &mut HashMap<String, String>,
        visiting: &mut HashSet<String>,
        depth: usize,
    ) {
        if depth >= MAX_STYLE_DEPTH || !visiting.insert(name.to_owned()) {
            return;
        }
        if let Some(style) = self.styles.get(name) {
            if let Some(base) = &style.base {
                self.resolve_named(base, state, out, visiting, depth + 1);
            }
            out.extend(style.properties.clone());
            for rule in &style.states {
                if state_matches(&rule.conditions, state) {
                    out.extend(rule.properties.clone());
                }
            }
        }
        visiting.remove(name);
    }
}

pub fn is_style_definition(name: &str) -> bool {
    name.contains('<')
}

fn leading_spaces(line: &str) -> usize {
    line.bytes().take_while(|byte| *byte == b' ').count()
}

fn expand_with(variables: &HashMap<String, String>, value: &str) -> String {
    let mut current = value.to_owned();
    for _ in 0..8 {
        let Some(start) = current.find('$') else {
            break;
        };
        let end = current[start..]
            .find(|c: char| c.is_whitespace() || c == ',' || c == ')')
            .map_or(current.len(), |offset| start + offset);
        let key = &current[start..end];
        let Some(replacement) = variables.get(key) else {
            break;
        };
        current.replace_range(start..end, replacement);
    }
    current
}

fn parse_conditions(name: &str) -> Vec<(String, bool)> {
    name.trim_end_matches(':')
        .split_whitespace()
        .map(|condition| {
            let enabled = !condition.starts_with('!');
            (
                condition
                    .trim_start_matches('$')
                    .trim_start_matches('!')
                    .to_ascii_lowercase(),
                enabled,
            )
        })
        .collect()
}

fn state_matches(conditions: &[(String, bool)], state: VisualState) -> bool {
    conditions.iter().all(|(name, expected)| {
        let actual = match name.as_str() {
            "hover" | "hovered" => state.hovered,
            "pressed" => state.pressed,
            "disabled" => state.disabled,
            "checked" => state.checked,
            "on" => state.on,
            "focus" | "focused" => state.focused,
            "dragging" => state.dragging,
            _ => false,
        };
        actual == *expected
    })
}

pub fn resolve_asset(root: &Path, source: &str) -> Option<PathBuf> {
    let clean = source.trim().trim_matches(['\'', '"']);
    if clean.is_empty() || clean.contains("..") {
        return None;
    }
    let relative = clean
        .trim_start_matches(['/', '\\'])
        .replace('/', std::path::MAIN_SEPARATOR_STR);
    let base = root.join("data").join(relative);
    if base.is_file() {
        return Some(base);
    }
    ["png", "jpg", "jpeg", "bmp"]
        .into_iter()
        .map(|extension| base.with_extension(extension))
        .find(|path| path.is_file())
}

fn collect_files(root: &Path, extension: &str, out: &mut Vec<PathBuf>, limit: usize) {
    if out.len() >= limit || !root.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        if out.len() >= limit || entry.file_type().is_ok_and(|kind| kind.is_symlink()) {
            break;
        }
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, extension, out, limit);
        } else if path
            .extension()
            .is_some_and(|value| value.eq_ignore_ascii_case(extension))
        {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_inheritance_variables_and_combined_states() {
        let mut book = StyleBook::default();
        book.parse("&tone: #112233\nBase < UIButton\n  color: $tone\n  size: 10 20\nChild < Base\n  $hover !disabled:\n    color: #abcdef\n");
        let doc = Document::parse(b"Child\n  id: button\n", true).unwrap();
        let normal = book.resolve(&doc, 0, VisualState::default());
        assert_eq!(normal.get("color"), "#112233");
        assert_eq!(normal.get("size"), "10 20");
        let hover = book.resolve(
            &doc,
            0,
            VisualState {
                hovered: true,
                ..Default::default()
            },
        );
        assert_eq!(hover.get("color"), "#abcdef");
    }
}
