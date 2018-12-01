use std::str;
use std::env::current_dir;
use std::clone::Clone;
use std::io::{Read, Seek, Write, SeekFrom, Error, ErrorKind, Cursor, BufReader};
use std::fs::{File,read_dir};
use std::path::{Path,PathBuf};
use std::collections::HashMap;
use std::iter::{Sum};

use armake::io::{Input};

mod preprocess_grammar {
    include!(concat!(env!("OUT_DIR"), "/preprocess_grammar.rs"));
}

#[derive(Clone, Debug)]
pub struct Definition {
    name: String,
    parameters: Option<Vec<String>>,
    value: Vec<Token>
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
pub struct Comment {
    newlines: u32,
}

#[derive(Debug)]
pub enum Token {
    RegularToken(String),
    MacroToken(Macro),
    CommentToken(Comment),
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

impl Clone for Comment {
    fn clone(&self) -> Comment {
        Comment {
            newlines: self.newlines
        }
    }
}

impl Clone for Token {
    fn clone(&self) -> Token {
        match self {
            Token::RegularToken(s) => Token::RegularToken(s.clone()),
            Token::MacroToken(m) => Token::MacroToken(m.clone()),
            Token::CommentToken(c) => Token::CommentToken(c.clone()),
            Token::ConcatToken => Token::ConcatToken,
        }
    }
}

impl Definition {
    pub fn value(&self, arguments: &Option<Vec<String>>, def_map: &HashMap<String,Definition>, stack: &Vec<Definition>) -> Result<Option<Vec<Token>>, String> {
        //println!("evaluating {}", self.name);
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

            for (param, arg) in params.iter().zip(args.iter()) {
                let mut tokens = preprocess_grammar::tokens(&arg).expect("Failed to parse macro argument");
                let stack: Vec<Definition> = Vec::new();
                tokens = Macro::resolve_all(&tokens, &def_map, &stack).expect("Failed to resolve macro arguments");

                local_map.insert(param.clone(), Definition {
                    name: param.clone(),
                    parameters: None,
                    value: tokens
                });
            }

            //println!("resolving {:?} {:?}", tokens, def_map);
            tokens = Macro::resolve_all(&tokens, &local_map, &stack_new)?;
            //println!("done");
        } else {
            //println!("resolving {:?} {:?}", tokens, def_map);
            tokens = Macro::resolve_all(&tokens, &def_map, &stack_new)?;
            //println!("done");
        }

        Ok(Some(tokens))
    }
}

impl Macro {
    pub fn resolve_pseudoargs(&self, def_map: &HashMap<String, Definition>, stack: &Vec<Definition>) -> Result<Vec<Token>, String> {
        let mut tokens: Vec<Token> = Vec::new();
        tokens.push(Token::RegularToken(self.name.clone()));

        if self.arguments.is_none() {
            return Ok(tokens);
        }

        //println!("resolving pseudoargs: {}", self.original);

        let (_, without_name) = self.original.split_at(self.name.len());
        let mut arg_tokens = preprocess_grammar::tokens(&without_name).expect("Failed to parse macro arguments.");

        arg_tokens = Macro::resolve_all(&arg_tokens, &def_map, &stack)?;
        for t in arg_tokens {
            tokens.push(t);
        }

        Ok(tokens)
    }

    pub fn resolve(&self, def_map: &HashMap<String, Definition>, stack: &Vec<Definition>) -> Result<Vec<Token>, String> {
        //println!("resolving: {}", self.name);
        let keys: Vec<&String> = def_map.keys().collect();
        //println!("keys: {:?}", keys);
        match def_map.get(&self.name) {
            Some(def) => {
                //println!("found");
                let value = def.value(&self.arguments, def_map, stack)?;
                if let Some(tokens) = value {
                    Ok(tokens)
                } else {
                    self.resolve_pseudoargs(def_map, stack)
                }
            },
            None => self.resolve_pseudoargs(def_map, stack)
        }
    }

    pub fn resolve_all(tokens: &Vec<Token>, def_map: &HashMap<String, Definition>, stack: &Vec<Definition>) -> Result<Vec<Token>, String> {
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
                Token::MacroToken(m) => {
                    output += &m.original;
                },
                Token::CommentToken(c) => {
                    newlines += c.newlines;
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

    content.split("\n").nth(0).unwrap().to_string()
}

fn pathsep() -> &'static str {
    if cfg!(windows) { "\\" } else { "/" }
}

fn matches_include_path(path: &PathBuf, include_path: &String) -> bool {
    let mut include_pathbuf = PathBuf::from(&include_path.replace("\\", pathsep()));

    //println!("{:?} {:?}", path, include_pathbuf);

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

        //println!("{:?}", parent);
        let relative = path.strip_prefix(parent).unwrap();
        let test_path = prefix_pathbuf.join(relative);

        if test_path == include_pathbuf {
            return true;
        }
    }

    false
}

fn search_directory(include_path: &String, directory: PathBuf) -> Option<PathBuf> {
    //println!("searching for {} in {:?}", include_path, directory);

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

    let mut include_pathbuf = PathBuf::from(&include_path.replace("\\", pathsep()));
    let direct_path = (&directory).to_str().unwrap().to_string() + &include_path.replace("\\", pathsep());
    let direct_pathbuf = PathBuf::from(direct_path);

    //println!("{:?}", direct_pathbuf);

    if direct_pathbuf.is_file() {
        return Some(direct_pathbuf);
    }

    None
}

pub fn find_include_file(include_path: &String, origin: Option<&PathBuf>, search_paths: &Vec<String>) -> Result<PathBuf, Error> {
    if include_path.chars().nth(0).unwrap() != '\\' {
        let mut path = PathBuf::from(&include_path);

        if let Some(origin_path) = origin {
            let absolute = PathBuf::from(&origin_path).canonicalize()?;
            let origin_dir = absolute.parent().unwrap();
            path = origin_dir.join(path);
        } else {
            path = current_dir()?.join(path);
        }

        if !path.is_file() {
            match origin {
                Some(origin_path) => Err(Error::new(ErrorKind::NotFound, format!("File \"{}\" included from \"{}\" doesn't exist.", include_path, origin_path.to_str().unwrap().to_string()))),
                None => Err(Error::new(ErrorKind::NotFound, format!("Included file \"{}\" doesn't exist.", include_path)))
            }
        } else {
            Ok(path)
        }
    } else {
        for search_path in search_paths {
            match search_directory(include_path, Path::new(&search_path).canonicalize()?) {
                Some(file_path) => { return Ok(file_path); },
                None => {}
            }
        }

        match origin {
            Some(origin_path) => Err(Error::new(ErrorKind::NotFound, format!("File \"{}\" included from \"{}\" not found.", include_path, origin_path.to_str().unwrap().to_string()))),
            None => Err(Error::new(ErrorKind::NotFound, format!("Included file \"{}\" not found.", include_path)))
        }
    }
}

fn preprocess_rec(input: String, origin: Option<PathBuf>, definition_map: &mut HashMap<String, Definition>, info: &mut PreprocessInfo) -> Result<String, Error> {
    let mut lines: Vec<Line> = preprocess_grammar::file(&input).expect("Failed to parse file");
    let mut output = String::from("");
    let mut original_lineno = 1;
    let mut level = 0;
    let mut level_true = 0;

    for mut line in lines {
        match line {
            Line::DirectiveLine(mut dir) => match dir {
                Directive::IncludeDirective(path) => {
                    //let import_tree = &mut info.import_tree;
                    //let includer = import_tree.get(&path);
                    //if let Some(path) = includer {
                    //    // @todo: complain
                    //}

                    let mut search_paths: Vec<String> = Vec::new();
                    search_paths.push(".".to_string());

                    let file_path = find_include_file(&path, origin.as_ref(), &search_paths)?;

                    info.import_stack.push(file_path.clone());

                    let mut content = String::new();
                    File::open(&file_path)?.read_to_string(&mut content)?;
                    //println!("file: {:?}", file_path);
                    let result = preprocess_rec(content, Some(file_path), definition_map, info)?;
                    //println!("file done.");

                    info.import_stack.pop();

                    output += &result;
                },
                Directive::DefineDirective(mut def) => {
                    if definition_map.remove(&def.name).is_some() {
                        // @todo: warn about redefine
                    }

                    original_lineno += u32::sum(def.value.iter().map(|t| match t {
                        Token::CommentToken(c) => c.newlines,
                        _ => 0
                    }));

                    //def.value = Macro::resolve_all(&def.value, &definition_map).expect("Failed to resolv macro value");

                    definition_map.insert(def.name.clone(), def);
                }
                Directive::UndefDirective(name) => {
                    definition_map.remove(&name);
                }
                Directive::IfDefDirective(name) => {
                    level += 1;
                    level_true += if definition_map.contains_key(&name) { 1 } else { 0 };
                }
                Directive::IfNDefDirective(name) => {
                    level += 1;
                    level_true += if definition_map.contains_key(&name) { 0 } else { 1 };
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
                let resolved = Macro::resolve_all(&tokens, &definition_map, &stack).expect("Failed to resolve macros");

                let (mut result, newlines) = Token::concat(&resolved);

                result = result.replace("\r\n", "\n").replace("\\\n", "");

                output += &result;
                output += "\n";

                original_lineno += newlines;
                info.line_origins.push((original_lineno, origin.clone()));
            }
        }
        original_lineno += 1;

        if level > 0 {
            // @todo: complain
        }
    }

    Ok(output)
}

pub fn preprocess(input: String, origin: Option<PathBuf>) -> Result<(String, PreprocessInfo), Error> {
    let mut info = PreprocessInfo {
        line_origins: Vec::new(),
        import_stack: Vec::new()
    };

    if let Some(ref path) = origin {
        info.import_stack.push(path.clone());
    }

    let mut def_map: HashMap<String, Definition> = HashMap::new();

    match preprocess_rec(input, origin, &mut def_map, &mut info) {
        Ok(result) => Ok((result, info)),
        Err(e) => Err(e)
    }
}

pub fn cmd_preprocess<I: Read, O: Write>(input: &mut I, output: &mut O, path: Option<PathBuf>) -> i32 {
    let mut buffer = String::new();
    input.read_to_string(&mut buffer).expect("Failed to read input file");

    let (result, info) = preprocess(buffer, path).expect("Failed to preprocess file");

    output.write_all(result.as_bytes());

    0
}
