use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockSpan {
    pub path: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HitError {
    UnmatchedClose { offset: usize },
    UnclosedBlock { paths: Vec<String> },
    BlockNotFound { path: String },
    AmbiguousBlock { path: String, count: usize },
    AmbiguousParameter { path: String, name: String, count: usize },
    InvalidMutation { detail: String },
}

impl fmt::Display for HitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnmatchedClose { offset } => write!(f, "unmatched closing block at byte {offset}"),
            Self::UnclosedBlock { paths } => {
                write!(f, "unclosed MOOSE block(s): {}", paths.join(", "))
            }
            Self::BlockNotFound { path } => write!(f, "block not found: {path}"),
            Self::AmbiguousBlock { path, count } => {
                write!(f, "expected one block {path:?}, found {count}")
            }
            Self::AmbiguousParameter { path, name, count } => {
                write!(f, "ambiguous parameter {path}/{name}: {count} assignments")
            }
            Self::InvalidMutation { detail } => f.write_str(detail),
        }
    }
}

impl std::error::Error for HitError {}

#[derive(Debug, Clone)]
pub struct MooseInput<'a> {
    text: &'a str,
    blocks: Vec<BlockSpan>,
}

impl<'a> MooseInput<'a> {
    pub fn parse(text: &'a str) -> Result<Self, HitError> {
        let mut stack: Vec<(String, usize)> = Vec::new();
        let mut blocks = Vec::new();
        let mut offset = 0usize;
        for segment in text.split_inclusive('\n') {
            let line = segment.trim_end_matches(['\r', '\n']);
            if let Some(token) = bracket_token(line) {
                if token.is_empty() || token == "../" {
                    if stack.is_empty() {
                        return Err(HitError::UnmatchedClose { offset });
                    }
                    let path = stack.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>().join("/");
                    let (_, start) = stack.pop().expect("stack checked non-empty");
                    blocks.push(BlockSpan { path, start, end: offset + segment.len() });
                } else {
                    let name = token.strip_prefix("./").unwrap_or(token).to_owned();
                    stack.push((name, offset));
                }
            }
            offset += segment.len();
        }
        if !stack.is_empty() {
            let paths = stack.iter().map(|(name, _)| name.clone()).collect();
            return Err(HitError::UnclosedBlock { paths });
        }
        Ok(Self { text, blocks })
    }

    pub fn text(&self) -> &'a str { self.text }
    pub fn blocks(&self) -> &[BlockSpan] { &self.blocks }

    pub fn find(&self, path: &str) -> Vec<&BlockSpan> {
        self.blocks.iter().filter(|block| block.path == path).collect()
    }

    pub fn unique(&self, path: &str) -> Result<&BlockSpan, HitError> {
        let matches = self.find(path);
        match matches.len() {
            0 => Err(HitError::BlockNotFound { path: path.to_owned() }),
            1 => Ok(matches[0]),
            count => Err(HitError::AmbiguousBlock { path: path.to_owned(), count }),
        }
    }

    pub fn direct_children(&self, parent: &str) -> Vec<String> {
        let prefix = format!("{parent}/");
        let depth = parent.matches('/').count() + 1;
        let mut result: Vec<_> = self.blocks.iter()
            .filter(|block| block.path.starts_with(&prefix) && block.path.matches('/').count() == depth)
            .map(|block| block.path.clone()).collect();
        result.sort();
        result
    }

    pub fn insert_before_close(&self, path: &str, fragment: &str) -> Result<String, HitError> {
        let span = self.unique(path)?;
        let block = &self.text[span.start..span.end];
        let trimmed = block.trim_end_matches(['\r', '\n']);
        let close_start = trimmed.rfind('\n').map_or(0, |i| i + 1);
        let close = trimmed[close_start..].trim();
        if close != "[]" && close != "[../]" {
            return Err(HitError::InvalidMutation { detail: format!("could not identify closing line for block {path:?}: {close:?}") });
        }
        let insert_at = span.start + close_start;
        let mut payload = fragment.to_owned();
        if !payload.is_empty() && !payload.ends_with('\n') { payload.push('\n'); }
        let out = format!("{}{}{}", &self.text[..insert_at], payload, &self.text[insert_at..]);
        Self::parse(&out)?;
        Ok(out)
    }
}

fn bracket_token(line: &str) -> Option<&str> {
    let body = line.trim_start();
    if !body.starts_with('[') {\n        return None;\n    }
    let close = body.find(']')?;
    let suffix = body[close + 1..].trim_start();
    if !suffix.is_empty() && !suffix.starts_with('#') {\n        return None;\n    }
    Some(body[1..close].trim())
}

pub fn append_top_level_block(text: &str, block_text: &str) -> Result<String, HitError> {
    let payload = block_text.trim_matches('\n');
    if payload.is_empty() { return Err(HitError::InvalidMutation { detail: "top-level block payload must not be empty".into() }); }
    let mut out = text.to_owned();
    if !out.is_empty() && !out.ends_with('\n') { out.push('\n'); }
    if !out.is_empty() && !out.ends_with("\n\n") { out.push('\n'); }
    out.push_str(payload); out.push('\n');
    MooseInput::parse(&out)?;
    Ok(out)
}

pub fn remove_block(text: &str, path: &str) -> Result<String, HitError> {
    let doc = MooseInput::parse(text)?;
    let span = doc.unique(path)?;
    let out = format!("{}{}", &text[..span.start], &text[span.end..]);
    MooseInput::parse(&out)?;
    Ok(out)
}

pub fn replace_block(text: &str, path: &str, replacement: &str) -> Result<String, HitError> {
    let doc = MooseInput::parse(text)?;
    let span = doc.unique(path)?;
    let mut payload = replacement.trim_end().to_owned(); payload.push('\n');
    let out = format!("{}{}{}", &text[..span.start], payload, &text[span.end..]);
    MooseInput::parse(&out)?;
    Ok(out)
}

fn parameter_matches(text: &str, span: &BlockSpan, name: &str) -> Vec<(usize, usize, String, String, String)> {
    let block = &text[span.start..span.end];
    let mut result = Vec::new();
    let mut local = 0usize;
    for segment in block.split_inclusive('\n') {
        let line = segment.trim_end_matches(['\r', '\n']);
        let code = line.split_once('#').map_or(line, |(before, _)| before);
        if let Some(eq) = code.find('=') {
            if code[..eq].trim() == name {
                let value = code[eq + 1..].trim().to_owned();
                let prefix_end = eq + 1 + code[eq + 1..].len() - code[eq + 1..].trim_start().len();
                let value_start = local + prefix_end;
                let value_end = value_start + value.len();
                result.push((value_start, value_end, value, line[..prefix_end].to_owned(), line[value_end-local..].to_owned()));
            }
        }
        local += segment.len();
    }
    result
}

pub fn get_parameter(text: &str, path: &str, name: &str) -> Result<Option<String>, HitError> {
    let doc = MooseInput::parse(text)?; let span = doc.unique(path)?;
    let matches = parameter_matches(text, span, name);
    if matches.len() > 1 { return Err(HitError::AmbiguousParameter { path: path.into(), name: name.into(), count: matches.len() }); }
    Ok(matches.first().map(|item| item.2.clone()))
}

pub fn upsert_parameter(text: &str, path: &str, name: &str, value: &str) -> Result<String, HitError> {
    let doc = MooseInput::parse(text)?; let span = doc.unique(path)?.clone();
    let matches = parameter_matches(text, &span, name);
    if matches.len() > 1 { return Err(HitError::AmbiguousParameter { path: path.into(), name: name.into(), count: matches.len() }); }
    if let Some((start, end, _, _, _)) = matches.first() {
        let a = span.start + start; let b = span.start + end;
        let out = format!("{}{}{}", &text[..a], value, &text[b..]); MooseInput::parse(&out)?; Ok(out)
    } else {
        doc.insert_before_close(path, &format!("  {name} = {value}"))
    }
}

pub fn remove_parameter(text: &str, path: &str, name: &str) -> Result<String, HitError> {
    let doc = MooseInput::parse(text)?; let span = doc.unique(path)?.clone();
    let block = &text[span.start..span.end];
    let mut hits = Vec::new(); let mut local = 0usize;
    for segment in block.split_inclusive('\n') {
        let line = segment.trim_end_matches(['\r','\n']);
        let code = line.split_once('#').map_or(line, |(before,_)| before);
        if code.find('=').is_some_and(|eq| code[..eq].trim() == name) { hits.push((local, local + segment.len())); }
        local += segment.len();
    }
    if hits.len() > 1 { return Err(HitError::AmbiguousParameter { path:path.into(), name:name.into(), count:hits.len() }); }
    let Some((start,end)) = hits.first().copied() else { return Ok(text.to_owned()); };
    let a=span.start+start; let b=span.start+end; let out=format!("{}{}", &text[..a], &text[b..]); MooseInput::parse(&out)?; Ok(out)
}
