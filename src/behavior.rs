use crate::document::Document;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionTemplate {
    Show,
    Hide,
    Toggle,
    Focus,
    Message,
    Custom,
}

impl ActionTemplate {
    pub const ALL: [Self; 6] = [
        Self::Show,
        Self::Hide,
        Self::Toggle,
        Self::Focus,
        Self::Message,
        Self::Custom,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Show => "Mostrar widget",
            Self::Hide => "Ocultar widget",
            Self::Toggle => "Alternar visibilidade",
            Self::Focus => "Focar widget",
            Self::Message => "Exibir mensagem",
            Self::Custom => "Expressão Lua",
        }
    }
    pub fn expression(self, target: &str, argument: &str) -> String {
        match self {
            Self::Show => format!("{target}:show()"),
            Self::Hide => format!("{target}:hide()"),
            Self::Toggle => format!("{target}:setVisible(not {target}:isVisible())"),
            Self::Focus => format!("{target}:focus()"),
            Self::Message => format!("displayInfoBox('NextGen', '{}')", escape_lua(argument)),
            Self::Custom => argument.trim().to_owned(),
        }
    }
}

pub fn events(document: &Document, node: usize) -> Vec<(String, String)> {
    document.nodes.get(node).map_or_else(Vec::new, |widget| {
        widget
            .properties
            .iter()
            .filter(|property| property.key.starts_with('@'))
            .map(|property| (property.key.clone(), property.value.clone()))
            .collect()
    })
}

pub fn widget_reference(id: &str) -> String {
    if id.is_empty() {
        "self".into()
    } else {
        format!(
            "self:getParent():recursiveGetChildById('{}')",
            escape_lua(id)
        )
    }
}

fn escape_lua(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace(['\r', '\n'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generates_safe_single_line_lua() {
        let value = ActionTemplate::Message.expression("self", "can't\nstop");
        assert_eq!(value, "displayInfoBox('NextGen', 'can\\'t stop')");
    }
}
