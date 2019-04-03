use std::env::current_dir;
use std::clone::Clone;
use std::io::{Read, Write, Error};
use std::fs::{File, read_dir};
use std::path::{Path, PathBuf, Component};
use std::collections::HashMap;
use std::iter::{Sum};

use crate::error::*;

pub mod preprocess_grammar {
    include!(concat!(env!("OUT_DIR"), "/preprocess_grammar.rs"));
}

#[derive(Clone, Debug)]
pub struct Definition {
    name: String,
    parameters: Option<Vec<String>>,
    value: Vec<Token>,
    local: bool
}

#[derive(Debug)]
pub enum Directive {
    IncludeDirective(String),
    DefineDirective(Definition),
    UndefDirective(String),
    IfDefDirective(String),
    IfNDefDirective(String),
    ElseDirective,
    EndIfDirective,
}

#[derive(Debug)]
pub struct Macro {
    name: String,
    arguments: Option<Vec<String>>,
    original: String,
    quoted: bool,
}

#[derive(Debug)]
pub enum Token {
    RegularToken(String),
    NewlineToken(String, u32),
    MacroToken(Macro),
    CommentToken(u32),
    ConcatToken
}

#[derive(Debug)]
pub enum Line {
    DirectiveLine(Directive),
    TokenLine(Vec<Token>),
}

#[derive(Debug)]
pub struct PreprocessInfo {
    pub line_origins: Vec<(u32, Option<PathBuf>)>,
    pub import_stack: Vec<PathBuf>
}

pub fn parse_macro(input: &str) -> Macro {
    let without_original: Macro = preprocess_grammar::macro_proper(input).unwrap();

    Macro {
        original: String::from(input),
        ..without_original
    }
}

impl Clone for Macro {
    fn clone(&self) -> Macro {
        Macro {
            name: self.name.clone(),
            arguments: self.arguments.clone(),
            original: self.original.clone(),
            quoted: self.quoted,
        }
    }
}

impl Clone for Token {
    fn clone(&self) -> Token {
        match self {
            Token::RegularToken(s) => Token::RegularToken(s.clone()),
            Token::NewlineToken(s, n) => Token::NewlineToken(s.clone(), *n),
            Token::MacroToken(m) => Token::MacroToken(m.clone()),
            Token::CommentToken(n) => Token::CommentToken(*n),
            Token::ConcatToken => Token::ConcatToken,
        }
    }
}

impl Definition {
    pub fn value(&self, arguments: &Option<Vec<String>>, def_map: &HashMap<String,Definition>, stack: &Vec<Definition>) -> Result<Option<Vec<Token>>, Error> {
        let params = self.parameters.clone().unwrap_or(Vec::new());
        let args = arguments.clone().unwrap_or(Vec::new());

        if params.len() != args.len() {
            return Ok(None);
        }

        let mut tokens = self.value.clone();

        if stack.iter().any(|d| d.name == self.name) {
            return Ok(Some(tokens));
        }

        let mut stack_new: Vec<Definition> = stack.clone();
        stack_new.push(self.clone());

        if params.len() > 0 {
            let mut local_map: HashMap<String,Definition> = HashMap::new();

            for (key, value) in def_map.iter() {
                local_map.insert(key.clone(), value.clone());
            }

            // @todo: handle these errors properly
            for (param, arg) in params.iter().zip(args.iter()) {
                let mut tokens = preprocess_grammar::tokens(&arg).expect("Failed to parse macro argument");
                let stack: Vec<Definition> = Vec::new();
                tokens = Macro::resolve_all(&tokens, &def_map, &stack).expect("Failed to resolve macro arguments");

                local_map.insert(param.clone(), Definition {
                    name: param.clone(),
                    parameters: None,
                    value: tokens,
                    local: true
                });
            }

            tokens = Macro::resolve_all(&tokens, &local_map, &stack_new)?;
        } else {
            tokens = Macro::resolve_all(&tokens, &def_map, &stack_new)?;
        }

        Ok(Some(tokens))
    }
}

impl Macro {
    pub fn resolve_pseudoargs(&self, def_map: &HashMap<String, Definition>, stack: &Vec<Definition>) -> Result<Vec<Token>, Error> {
        let mut tokens: Vec<Token> = Vec::new();
        tokens.push(Token::RegularToken(self.name.clone()));

        if self.arguments.is_none() {
            return Ok(tokens);
        }

        let (_, without_name) = self.original.split_at(self.name.len());
        let mut arg_tokens = preprocess_grammar::tokens(&without_name).expect("Failed to parse macro arguments.");

        arg_tokens = Macro::resolve_all(&arg_tokens, &def_map, &stack)?;
        for t in arg_tokens {
            tokens.push(t);
        }

        Ok(tokens)
    }

    pub fn resolve(&self, def_map: &HashMap<String, Definition>, stack: &Vec<Definition>) -> Result<Vec<Token>, Error> {
        match def_map.get(&self.name) {
            Some(def) => {
                let value = def.value(&self.arguments, def_map, stack)?;

                if !def.local && self.quoted {
                    // @todo: complain
                }

                if let Some(tokens) = value {
                    if self.quoted {
                        let (concatted, newlines) = Token::concat(&tokens);
                        let mut tokens: Vec<Token> = Vec::new();
                        tokens.push(Token::NewlineToken(format!("\"{}\"", concatted.trim()), newlines));
                        Ok(tokens)
                    } else {
                        Ok(tokens)
                    }
                } else {
                    self.resolve_pseudoargs(def_map, stack)
                }
            },
            None => self.resolve_pseudoargs(def_map, stack)
        }
    }

    pub fn resolve_all(tokens: &Vec<Token>, def_map: &HashMap<String, Definition>, stack: &Vec<Definition>) -> Result<Vec<Token>, Error> {
        let mut result: Vec<Token> = Vec::new();

        for token in tokens {
            match token {
                Token::MacroToken(ref m) => {
                    let resolved = m.resolve(def_map, stack)?;
                    for t in resolved {
                        result.push(t);
                    }
                },
                _ => {
                    result.push(token.clone());
                }
            }
        }

        Ok(result)
    }
}

impl Token {
    pub fn concat(tokens: &Vec<Token>) -> (String, u32) {
        let mut output = String::new();
        let mut newlines = 0;

        for token in tokens {
            match token {
                Token::RegularToken(s) => {
                    output += &s;
                },
                Token::NewlineToken(s,  n) => {
                    output += &s;
                    newlines += n;
                },
                Token::MacroToken(m) => {
                    output += &m.original;
                },
                Token::CommentToken(n) => {
                    newlines += n;
                },
                _ => {}
            }
        }

        (output, newlines)
    }
}

fn read_prefix(prefix_path: &Path) -> String {
    let mut content = String::new();
    File::open(prefix_path).unwrap().read_to_string(&mut content).unwrap();

    content.replace("\r\n","\n").split("\n").nth(0).unwrap().to_string()
}

pub fn pathsep() -> &'static str {
    if cfg!(windows) { "\\" } else { "/" }
}

fn matches_include_path(path: &PathBuf, include_path: &String) -> bool {
    let include_pathbuf = PathBuf::from(&include_path.replace("\\", pathsep()));

    if path.file_name() != include_pathbuf.file_name() { return false; }

    for parent in path.ancestors() {
        if parent.is_file() { continue; }

        let prefixpath = parent.join("$PBOPREFIX$");
        if !prefixpath.is_file() { continue; }

        let mut prefix = read_prefix(&prefixpath);

        prefix = if prefix.len() > 0 && prefix.chars().nth(0).unwrap() != '\\' {
            format!("\\{}", prefix)
        } else {
            prefix
        };

        let prefix_pathbuf = PathBuf::from(prefix.replace("\\", pathsep()));

        let relative = path.strip_prefix(parent).unwrap();
        let test_path = prefix_pathbuf.join(relative);

        if test_path == include_pathbuf {
            return true;
        }
    }

    false
}

fn search_directory(include_path: &String, directory: PathBuf) -> Option<PathBuf> {
    for entry in read_dir(&directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            if path.file_name().unwrap() == ".git" {
                continue;
            }

            match search_directory(include_path, path) {
                Some(path) => { return Some(path); }
                None => {}
            }
        } else {
            if matches_include_path(&path, include_path) {
                return Some(path);
            }
        }
    }

    let direct_path = (&directory).to_str().unwrap().to_string() + &include_path.replace("\\", pathsep());
    let direct_pathbuf = PathBuf::from(direct_path);

    if direct_pathbuf.is_file() {
        return Some(direct_pathbuf);
    }

    None
}

fn canonicalize(path: PathBuf) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                result.pop();
            },
            _ => {
                result.push(component);
            }
        }
    }
    result
}

pub fn find_include_file(include_path: &String, origin: Option<&PathBuf>, search_paths: &Vec<PathBuf>) -> Result<PathBuf, Error> {
    if include_path.chars().nth(0).unwrap() != '\\' {
        let mut path = PathBuf::from(include_path.replace("\\", pathsep()));

        if let Some(origin_path) = origin {
            let absolute = PathBuf::from(&origin_path).canonicalize()?;
            let origin_dir = absolute.parent().unwrap();
            path = origin_dir.join(path);
        } else {
            path = current_dir()?.join(path);
        }

        let absolute = canonicalize(path);

        if !absolute.is_file() {
            match origin {
                Some(origin_path) => Err(error!("File \"{}\" included from \"{}\" doesn't exist.", include_path, origin_path.to_str().unwrap().to_string())),
                None => Err(error!("Included file \"{}\" doesn't exist.", include_path))
            }
        } else {
            Ok(absolute)
        }
    } else {
        for search_path in search_paths {
            match search_directory(include_path, search_path.canonicalize()?) {
                Some(file_path) => { return Ok(file_path); },
                None => {}
            }
        }

        match origin {
            Some(origin_path) => Err(error!("File \"{}\" included from \"{}\" not found.", include_path, origin_path.to_str().unwrap().to_string())),
            None => Err(error!("Included file \"{}\" not found.", include_path))
        }
    }
}

struct PreprocessHolder<'a> {
    input: String,
    origin: Option<PathBuf>,
    definition_map: HashMap<String, Definition>,
    info: PreprocessInfo,
    includefolders: &'a Vec<PathBuf>,
    line: Iterator<Item = Line>,
}

impl<'a> Iterator for PreprocessHolder<'a> {
    type Item = HashMap<String, Definition>;
    fn next(&mut self) -> Option<Self::Item> {
        let line = self.line.next(); // Will this actually change self.line() for the next step?
        None
    }
}

fn line_muncher(line:Line, refoutput: &mut String, reforiginal_lineno: &mut u32, reflevel: &mut u32, reflevel_true: &mut u32, input: String, origin: Option<PathBuf>, definition_map: &mut HashMap<String, Definition>, info: &mut PreprocessInfo, includefolders: &Vec<PathBuf>) -> Result<String, Error> {
        let level = *reflevel;
        let level_true = *reflevel_true;
        let original_lineno = *reforiginal_lineno;
        let output = *refoutput;
        match line {
            Line::DirectiveLine(dir) => match dir {
                Directive::IncludeDirective(path) => {
                    if level > level_true { return Ok(output); }

                    //let import_tree = &mut info.import_tree;
                    //let includer = import_tree.get(&path);
                    //if let Some(path) = includer {
                    //    // @todo: complain
                    //}

                    let file_path = find_include_file(&path, origin.as_ref(), includefolders)?;

                    info.import_stack.push(file_path.clone());

                    let mut content = String::new();
                    File::open(&file_path)?.read_to_string(&mut content)?;
                    let result = preprocess_rec(content, Some(file_path), definition_map, info, includefolders).prepend_error(format!("Failed to preprocess include \"{}\":", path))?;

                    info.import_stack.pop();

                    output += &result;
                },
                Directive::DefineDirective(def) => {
                    original_lineno += u32::sum(def.value.iter().map(|t| match t {
                        Token::NewlineToken(_s, n) => *n,
                        Token::CommentToken(n) => *n,
                        _ => 0
                    }));

                    if level > level_true { return Ok(output); }

                    if definition_map.remove(&def.name).is_some() {
                        // @todo: warn about redefine
                    }

                    definition_map.insert(def.name.clone(), def);
                }
                Directive::UndefDirective(name) => {
                    if level > level_true { return Ok(output); }

                    definition_map.remove(&name);
                }
                Directive::IfDefDirective(name) => {
                    level_true += if level_true == level && definition_map.contains_key(&name) { 1 } else { 0 };
                    level += 1;
                }
                Directive::IfNDefDirective(name) => {
                    level_true += if level_true == level && !definition_map.contains_key(&name) { 1 } else { 0 };
                    level += 1;
                }
                Directive::ElseDirective => {
                    if level_true + 1 == level {
                        level_true = level;
                    } else if level_true == level {
                        level_true -= 1;
                    }
                }
                Directive::EndIfDirective => {
                    assert!(level > 0);
                    level -= 1;
                    if level_true > level {
                        level_true -= 1;
                    }
                }
            },
            Line::TokenLine(tokens) => {
                let stack: Vec<Definition> = Vec::new();
                let resolved = Macro::resolve_all(&tokens, &definition_map, &stack).prepend_error("Failed to resolve macros:")?;

                let (mut result, newlines) = Token::concat(&resolved);
                result = result.replace("\r\n", "\n").replace("\\\n", "");
                original_lineno += newlines;

                if level > level_true { return Ok(output); }

                output += &result;
                output += "\n";

                info.line_origins.push((original_lineno, origin.clone()));
            }
        }
        original_lineno += 1;

        if level > 0 {
            // @todo: complain
        }

        Ok(output)
}

fn preprocess_rec(input: String, origin: Option<PathBuf>, definition_map: &mut HashMap<String, Definition>, info: &mut PreprocessInfo, includefolders: &Vec<PathBuf>) -> Result<String, Error> {
    let lines = preprocess_grammar::file(&input).format_error(&origin, &input)?;
    let mut output = String::from("");
    let mut original_lineno = 1;
    let mut level = 0;
    let mut level_true = 0;

    // lines is already an iterator - easy
    for line in lines {
        output += &line_muncher(line, &mut output, &mut original_lineno, &mut level, &mut level_true, input, origin, definition_map, info, includefolders).unwrap()
        // this needs to be a function f(line,state)
        //
        // iterator has function next(&mut self)
        //
        // what I want is to return the definition_map for each original line
        //
        // iterator can mutate internal result (but def_map will probably be more up to date)
    }

    Ok(output)
}

pub fn preprocess(mut input: String, origin: Option<PathBuf>, includefolders: &Vec<PathBuf>) -> Result<(String, PreprocessInfo), Error> {
    if input[..3].as_bytes() == &[0xef,0xbb,0xbf] {
        input = input[3..].to_string();
    }

    let mut info = PreprocessInfo {
        line_origins: Vec::new(),
        import_stack: Vec::new()
    };

    if let Some(ref path) = origin {
        info.import_stack.push(path.clone());
    }

    let mut def_map: HashMap<String, Definition> = HashMap::new();

    match preprocess_rec(input, origin, &mut def_map, &mut info, includefolders) {
        Ok(result) => Ok((result, info)),
        Err(e) => Err(e)
    }
}

pub fn cmd_preprocess<I: Read, O: Write>(input: &mut I, output: &mut O, path: Option<PathBuf>, includefolders: &Vec<PathBuf>) -> Result<(), Error> {
    let mut buffer = String::new();
    input.read_to_string(&mut buffer).expect("Failed to read input file");

    let (result, _) = preprocess(buffer, path, includefolders).expect("Failed to preprocess file");

    output.write_all(result.as_bytes()).expect("Failed to write output");

    Ok(())
}
