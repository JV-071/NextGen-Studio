use encoding_rs::WINDOWS_1252;
use std::{
    collections::VecDeque,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
pub type Result<T> = std::result::Result<T, String>;
pub const MAX_BYTES: usize = 8 * 1024 * 1024;
#[derive(Clone, Debug)]
pub struct Property {
    pub line: usize,
    pub key: String,
    pub value: String,
    pub block: bool,
}
#[derive(Clone, Debug)]
pub struct Node {
    pub line: usize,
    pub end: usize,
    pub parent: Option<usize>,
    pub indent: usize,
    pub name: String,
    pub widget: bool,
    pub properties: Vec<Property>,
}
#[derive(Clone)]
struct Line {
    body: Vec<u8>,
    eol: Vec<u8>,
}
#[derive(Clone)]
pub struct Document {
    lines: Vec<Line>,
    bom: bool,
    utf8: bool,
    pub nodes: Vec<Node>,
    pub issues: Vec<String>,
    pub path: Option<PathBuf>,
    saved: Vec<u8>,
    pub is_otui: bool,
}
fn decode(bytes: &[u8], utf8: bool) -> String {
    if utf8 {
        String::from_utf8_lossy(bytes).into_owned()
    } else {
        WINDOWS_1252.decode(bytes).0.into_owned()
    }
}
fn valid_name(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .enumerate()
            .all(|(i, c)| c.is_ascii_alphabetic() || c == b'_' || (i > 0 && c.is_ascii_digit()))
}
pub fn read_bounded(path: &Path) -> Result<Vec<u8>> {
    let f = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    f.take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_BYTES {
        return Err("Arquivo maior que 8 MiB.".into());
    }
    Ok(bytes)
}
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("Destino é um link simbólico.".into());
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    tmp.write_all(data)
        .and_then(|_| tmp.as_file().sync_all())
        .map_err(|e| e.to_string())?;
    if let Ok(meta) = fs::metadata(path) {
        tmp.as_file()
            .set_permissions(meta.permissions())
            .map_err(|e| e.to_string())?;
    }
    tmp.persist(path).map_err(|e| e.error.to_string())?;
    #[cfg(unix)]
    fs::File::open(parent)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(())
}
impl Document {
    pub fn parse(bytes: &[u8], is_otui: bool) -> Result<Self> {
        if bytes.len() > MAX_BYTES || bytes.contains(&0) {
            return Err("Arquivo binário ou maior que 8 MiB.".into());
        }
        let bom = bytes.starts_with(&[239, 187, 191]);
        let data = if bom { &bytes[3..] } else { bytes };
        let utf8 = std::str::from_utf8(data).is_ok();
        if bom && !utf8 {
            return Err("UTF-8 inválido com BOM.".into());
        }
        let mut lines = Vec::new();
        for raw in data.split_inclusive(|c| *c == b'\n') {
            let (body, eol) = if raw.ends_with(b"\r\n") {
                (&raw[..raw.len() - 2], b"\r\n".as_slice())
            } else if raw.ends_with(b"\n") {
                (&raw[..raw.len() - 1], b"\n".as_slice())
            } else {
                (raw, b"".as_slice())
            };
            if !raw.is_empty() {
                lines.push(Line {
                    body: body.to_vec(),
                    eol: eol.to_vec(),
                });
            }
        }
        let mut d = Self {
            lines,
            bom,
            utf8,
            nodes: vec![],
            issues: vec![],
            path: None,
            saved: bytes.to_vec(),
            is_otui,
        };
        d.index();
        Ok(d)
    }
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = read_bounded(path)?;
        let mut d = Self::parse(
            &bytes,
            path.extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("otui")),
        )?;
        d.path = Some(fs::canonicalize(path).map_err(|e| e.to_string())?);
        Ok(d)
    }
    pub fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        if self.bom {
            out.extend([239, 187, 191]);
        }
        for l in &self.lines {
            out.extend(&l.body);
            out.extend(&l.eol);
        }
        out
    }
    pub fn text(&self) -> String {
        let bytes = self.bytes();
        decode(if self.bom { &bytes[3..] } else { &bytes }, self.utf8)
    }
    pub fn encoding(&self) -> &str {
        if self.utf8 { "UTF-8" } else { "Windows-1252" }
    }
    pub fn dirty(&self) -> bool {
        self.bytes() != self.saved
    }
    pub fn mark_new(&mut self) {
        self.saved.clear();
        self.path = None;
    }
    pub fn replace_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        let mut next = Self::parse(bytes, self.is_otui)?;
        next.path = self.path.clone();
        next.saved = self.saved.clone();
        *self = next;
        Ok(())
    }
    fn encode(&self, text: &str) -> Result<Vec<u8>> {
        if self.utf8 {
            return Ok(text.as_bytes().to_vec());
        }
        let (bytes, _, bad) = WINDOWS_1252.encode(text);
        if bad {
            Err("Caractere não representável em Windows-1252.".into())
        } else {
            Ok(bytes.into_owned())
        }
    }
    pub fn replace_text(&mut self, text: &str) -> Result<()> {
        let mut bytes = if self.bom {
            vec![239, 187, 191]
        } else {
            vec![]
        };
        bytes.extend(self.encode(text)?);
        self.replace_bytes(&bytes)
    }
    fn index(&mut self) {
        self.nodes.clear();
        self.issues.clear();
        if !self.is_otui {
            return;
        }
        let mut stack: Vec<usize> = vec![];
        let mut block: Option<usize> = None;
        for (i, line) in self.lines.iter().enumerate() {
            let text = decode(&line.body, self.utf8);
            let trimmed = text.trim();
            let indent = line.body.iter().take_while(|c| **c == b' ').count();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
                continue;
            }
            if block.is_some_and(|b| indent > b) {
                continue;
            }
            block = None;
            if line.body.get(indent) == Some(&b'\t') {
                self.issues.push(format!(
                    "Linha {}: tabulação na indentação; edição estrutural bloqueada.",
                    i + 1
                ));
            }
            while stack
                .last()
                .is_some_and(|n| self.nodes[*n].indent >= indent)
            {
                let n = stack.pop().unwrap();
                self.nodes[n].end = i;
            }
            if let Some((key, val)) = trimmed.split_once(':') {
                let Some(&parent) = stack.last() else {
                    self.issues
                        .push(format!("Linha {}: propriedade sem pai.", i + 1));
                    continue;
                };
                let (key, val) = (key.trim(), val.trim());
                let multi = matches!(val, "|" | "|-" | "|+");
                self.nodes[parent].properties.push(Property {
                    line: i,
                    key: key.into(),
                    value: val.into(),
                    block: multi || val.is_empty(),
                });
                if multi {
                    block = Some(indent);
                } else if val.is_empty() {
                    let n = self.nodes.len();
                    self.nodes.push(Node {
                        line: i,
                        end: self.lines.len(),
                        parent: Some(parent),
                        indent,
                        name: key.into(),
                        widget: false,
                        properties: vec![],
                    });
                    stack.push(n);
                }
            } else {
                let n = self.nodes.len();
                self.nodes.push(Node {
                    line: i,
                    end: self.lines.len(),
                    parent: stack.last().copied(),
                    indent,
                    name: trimmed.into(),
                    widget: !trimmed.starts_with('$'),
                    properties: vec![],
                });
                stack.push(n);
            }
        }
    }
    pub fn value(&self, n: usize, key: &str) -> &str {
        self.nodes
            .get(n)
            .and_then(|n| n.properties.iter().find(|p| p.key == key))
            .map_or("", |p| p.value.as_str())
    }
    fn structural(&self) -> Result<()> {
        if self.issues.iter().any(|s| s.contains("tabulação")) {
            Err("Resolva a indentação com tabulações no código.".into())
        } else {
            Ok(())
        }
    }
    pub fn set(&mut self, n: usize, key: &str, val: &str) -> Result<()> {
        self.structural()?;
        if key.is_empty()
            || !key
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_-.!@*".contains(&c))
            || val.contains(['\n', '\r', '\0'])
            || val.trim().is_empty()
        {
            return Err("Propriedade ou valor inválido.".into());
        }
        let node = self.nodes.get(n).ok_or("Selecione um elemento.")?.clone();
        let data = self.encode(val)?;
        if self.bytes().len() + data.len() + key.len() + 64 > MAX_BYTES {
            return Err("Documento excederia 8 MiB.".into());
        }
        let matches: Vec<_> = node.properties.iter().filter(|p| p.key == key).collect();
        if matches.len() > 1 {
            return Err("Propriedade duplicada; resolva no código.".into());
        }
        if let Some(p) = matches.first() {
            if p.block {
                return Err("Bloco composto: use Código.".into());
            }
            let line = &mut self.lines[p.line].body;
            let mut at = line.iter().position(|c| *c == b':').unwrap() + 1;
            while line.get(at).is_some_and(|c| *c == b' ' || *c == b'\t') {
                at += 1;
            }
            line.truncate(at);
            line.extend(data);
        } else {
            let eol = self
                .lines
                .iter()
                .find(|l| !l.eol.is_empty())
                .map_or(b"\n".to_vec(), |l| l.eol.clone());
            if self.lines[node.line].eol.is_empty() {
                self.lines[node.line].eol = eol.clone();
            }
            self.lines.insert(
                node.line + 1,
                Line {
                    body: [
                        vec![b' '; node.indent + 2],
                        key.as_bytes().to_vec(),
                        b": ".to_vec(),
                        data,
                    ]
                    .concat(),
                    eol,
                },
            );
        }
        self.index();
        Ok(())
    }
    pub fn add(&mut self, parent: Option<usize>, kind: &str, id: &str) -> Result<()> {
        self.structural()?;
        if !valid_name(kind) || !valid_name(id) {
            return Err("Tipo ou ID inválido.".into());
        }
        if self
            .nodes
            .iter()
            .enumerate()
            .any(|(n, _)| self.value(n, "id") == id)
        {
            return Err("ID já existe.".into());
        }
        let (at, indent) = if let Some(p) = parent {
            let n = self.nodes.get(p).ok_or("Pai inválido.")?;
            if !n.widget {
                return Err("Selecione um widget como pai.".into());
            }
            (n.end, n.indent + 2)
        } else {
            (self.lines.len(), 0)
        };
        if self.bytes().len() + indent * 3 + id.len() + kind.len() + 80 > MAX_BYTES {
            return Err("Documento excederia 8 MiB.".into());
        }
        let eol = self
            .lines
            .iter()
            .find(|l| !l.eol.is_empty())
            .map_or(b"\n".to_vec(), |l| l.eol.clone());
        if at > 0 && self.lines[at - 1].eol.is_empty() {
            self.lines[at - 1].eol = eol.clone();
        }
        for (offset, text) in [
            format!("{}{kind}", " ".repeat(indent)),
            format!("{}id: {id}", " ".repeat(indent + 2)),
            format!("{}size: 120 32", " ".repeat(indent + 2)),
        ]
        .into_iter()
        .enumerate()
        {
            self.lines.insert(
                at + offset,
                Line {
                    body: text.into_bytes(),
                    eol: eol.clone(),
                },
            );
        }
        self.index();
        Ok(())
    }
    pub fn remove_property(&mut self, n: usize, key: &str) -> Result<()> {
        self.structural()?;
        let node = self.nodes.get(n).ok_or("Seleção inválida.")?;
        let matches: Vec<_> = node.properties.iter().filter(|p| p.key == key).collect();
        if matches.len() != 1 {
            return Err("Propriedade ausente ou duplicada.".into());
        }
        if matches[0].block {
            return Err("Remova blocos compostos no código.".into());
        }
        let line = matches[0].line;
        self.lines.remove(line);
        self.index();
        Ok(())
    }
    pub fn remove(&mut self, n: usize) -> Result<()> {
        self.structural()?;
        let node = self.nodes.get(n).ok_or("Seleção inválida.")?;
        if !node.widget {
            return Err("Selecione um widget.".into());
        }
        let start = node.line;
        let mut end = node.end;
        while end > start + 1 {
            let s = decode(&self.lines[end - 1].body, self.utf8);
            let t = s.trim();
            if t.is_empty() || t.starts_with('#') || t.starts_with("//") {
                end -= 1;
            } else {
                break;
            }
        }
        self.lines.drain(start..end);
        self.index();
        Ok(())
    }
    pub fn save(&mut self, target: &Path, overwrite: bool) -> Result<()> {
        let same = self
            .path
            .as_ref()
            .is_some_and(|p| fs::canonicalize(target).is_ok_and(|q| q == *p));
        if same && read_bounded(target).ok().as_ref() != Some(&self.saved) {
            return Err("Arquivo mudou fora do editor. Reabra ou salve uma cópia.".into());
        }
        if target.exists() && !same && !overwrite {
            return Err("Destino já existe.".into());
        }
        if target.exists() {
            let original = read_bounded(target)?;
            let mut backup = target.as_os_str().to_os_string();
            backup.push(".bak");
            atomic_write(Path::new(&backup), &original)?;
        }
        let bytes = self.bytes();
        atomic_write(target, &bytes)?;
        self.path = Some(fs::canonicalize(target).map_err(|e| e.to_string())?);
        self.saved = bytes;
        Ok(())
    }
}
struct Change {
    at: usize,
    old: Vec<u8>,
    new: Vec<u8>,
    label: String,
}
#[derive(Default)]
pub struct History {
    undo: VecDeque<Change>,
    redo: Vec<Change>,
    bytes: usize,
}
impl History {
    pub fn apply(
        &mut self,
        d: &mut Document,
        label: &str,
        op: impl FnOnce(&mut Document) -> Result<()>,
    ) -> Result<()> {
        let before = d.bytes();
        let mut next = d.clone();
        op(&mut next)?;
        let after = next.bytes();
        if before == after {
            return Ok(());
        }
        for c in self.redo.drain(..) {
            self.bytes -= c.old.len() + c.new.len();
        }
        let at = before
            .iter()
            .zip(&after)
            .take_while(|(a, b)| a == b)
            .count();
        let suffix = before[at..]
            .iter()
            .rev()
            .zip(after[at..].iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        let c = Change {
            at,
            old: before[at..before.len() - suffix].to_vec(),
            new: after[at..after.len() - suffix].to_vec(),
            label: label.into(),
        };
        self.bytes += c.old.len() + c.new.len();
        self.undo.push_back(c);
        while self.undo.len() > 150 || self.bytes > 32 * 1024 * 1024 {
            if let Some(c) = self.undo.pop_front() {
                self.bytes -= c.old.len() + c.new.len();
            } else {
                break;
            }
        }
        *d = next;
        Ok(())
    }
    pub fn undo(&mut self, d: &mut Document) -> Result<()> {
        if let Some(c) = self.undo.back() {
            let mut b = d.bytes();
            b.splice(c.at..c.at + c.new.len(), c.old.iter().copied());
            d.replace_bytes(&b)?;
            let c = self.undo.pop_back().unwrap();
            self.redo.push(c);
        }
        Ok(())
    }
    pub fn redo(&mut self, d: &mut Document) -> Result<()> {
        if let Some(c) = self.redo.last() {
            let mut b = d.bytes();
            b.splice(c.at..c.at + c.old.len(), c.new.iter().copied());
            d.replace_bytes(&b)?;
            let c = self.redo.pop().unwrap();
            self.undo.push_back(c);
        }
        Ok(())
    }
    pub fn labels(&self) -> Vec<&str> {
        self.undo
            .iter()
            .rev()
            .take(20)
            .map(|c| c.label.as_str())
            .collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_roundtrip_and_minimal_patch() {
        let b=b"\xef\xbb\xbf# keep\r\nPanel\r\n  text: old\r\n  @onClick: |\r\n    foo('a:b')\r\n  $hover:\r\n    color: red\r\n  Button\r\n    text: child";
        let mut d = Document::parse(b, true).unwrap();
        assert_eq!(d.bytes(), b);
        assert_eq!(d.nodes.len(), 3);
        d.set(2, "text", "new").unwrap();
        assert_eq!(
            d.bytes(),
            String::from_utf8_lossy(b)
                .replace("text: child", "text: new")
                .into_bytes()
        );
        assert!(d.set(0, "@onClick", "oops").is_err());
    }
    #[test]
    fn cp1252_and_invalid_edits() {
        let mut d = Document::parse(b"Label\n  text: Caf\xe9\n", true).unwrap();
        assert_eq!(d.encoding(), "Windows-1252");
        let b = d.bytes();
        assert!(d.set(0, "text", "😀").is_err());
        assert_eq!(d.bytes(), b);
        d.set(0, "text", "Ação").unwrap();
        assert!(d.bytes().contains(&0xe7));
        assert!(d.replace_bytes(b"\xef\xbb\xbf\xff").is_err());
        assert_eq!(d.encoding(), "Windows-1252");
    }
    #[test]
    fn structure_and_history() {
        let mut d=Document::parse(b"Panel\n  layout:\n    type: verticalBox\n  Button\n    id: first\n  Button\n    id: second\n",true).unwrap();
        assert_eq!(d.nodes.len(), 4);
        let before = d.bytes();
        let mut h = History::default();
        h.apply(&mut d, "Remove", |d| d.remove(2)).unwrap();
        assert!(!d.text().contains("first"));
        h.undo(&mut d).unwrap();
        assert_eq!(before, d.bytes());
        h.redo(&mut d).unwrap();
        assert!(!d.text().contains("first"));
        h.apply(&mut d, "Add", |d| d.add(Some(0), "Label", "third"))
            .unwrap();
        assert!(d.add(Some(0), "Label", "third").is_err());
    }
    #[test]
    fn invalid_input_and_tabs() {
        assert!(Document::parse(b"a\0b", true).is_err());
        assert!(Document::parse(&vec![b'a'; MAX_BYTES + 1], true).is_err());
        let mut d = Document::parse(b"Label\n\ttext: x\n", true).unwrap();
        assert!(d.set(0, "text", "y").is_err());
        assert!(d.remove(0).is_err());
    }
    #[test]
    fn save_backup_conflict() {
        let temp = tempfile::tempdir().unwrap();
        let p = temp.path().join("a.otui");
        let mut d = Document::parse(b"Label\n  text: old\n", true).unwrap();
        d.save(&p, false).unwrap();
        d.set(0, "text", "new").unwrap();
        d.save(&p, false).unwrap();
        assert_eq!(
            fs::read(p.with_extension("otui.bak")).unwrap(),
            b"Label\n  text: old\n"
        );
        fs::write(&p, b"external").unwrap();
        assert!(d.save(&p, true).is_err());
        assert_eq!(fs::read(&p).unwrap(), b"external");
    }
    #[test]
    fn source_keeps_encoding_and_paths() {
        let mut d = Document::parse(b"local x = 1\r\n", false).unwrap();
        assert!(d.nodes.is_empty());
        let mut h = History::default();
        h.apply(&mut d, "code", |d| d.replace_text("local x = 2\r\n"))
            .unwrap();
        h.undo(&mut d).unwrap();
        assert_eq!(d.bytes(), b"local x = 1\r\n");
    }
}
