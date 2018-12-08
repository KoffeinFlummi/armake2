use std::io::{Read, Write, Cursor, Error, ErrorKind};
use std::fs::{File, create_dir_all, copy, remove_dir_all};
use std::collections::{HashSet};
use std::path::{PathBuf};
use std::env::{var, temp_dir};
use std::process::{Command, Stdio};

#[cfg(windows)]
use winreg::RegKey;
#[cfg(windows)]
use winreg::enums::*;

use armake::error::*;
use armake::p3d::*;
use armake::preprocess::*;

#[cfg(windows)]
fn find_binarize_exe() -> Result<PathBuf, Error> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let binarize = hkcu.open_subkey("Software\\Bohemia Interactive\\binarize")?;
    let value: String = binarize.get_value("path")?;

    Ok(PathBuf::from(value).join("binarize.exe"))
}

#[cfg(unix)]
fn find_binarize_exe() -> Result<PathBuf, Error> {
    unreachable!();
}

fn extract_dependencies(input: &PathBuf) -> Result<Vec<String>, Error> {
    let mut file = File::open(input)?;
    let p3d = P3D::read(&mut file, true)?;

    let mut set: HashSet<String> = HashSet::new();

    for lod in p3d.lods {
        for face in lod.faces {
            if face.texture.len() > 0 && face.texture.chars().nth(0).unwrap() != '#' {
                set.insert(face.texture);
            }
            if face.material.len() > 0 && face.material.chars().nth(0).unwrap() != '#' {
                set.insert(face.material);
            }
        }
    }

    Ok(set.iter().map(|s| s.clone()).collect())
}

fn create_temp_directory(name: &String) -> Result<PathBuf, Error> {
    let dir = temp_dir();
    let mut i = 0;

    let mut path;
    loop {
        path = dir.join(format!("armake_{}_{}", name, i));
        if !path.exists() { break; }

        i += 1;
    }

    create_dir_all(&path)?;

    Ok(path)
}

pub fn binarize(input: &PathBuf, includefolders: &Vec<PathBuf>) -> Result<Cursor<Box<[u8]>>, Error> {
    if !cfg!(windows) {
        return Err(Error::new(ErrorKind::Other, "binarize.exe is only available on windows. Use rapify to binarize configs."));
    }

    let binarize_exe = find_binarize_exe().prepend_error("Failed to find BI's binarize.exe:")?;
    if !binarize_exe.exists() {
        return Err(Error::new(ErrorKind::Other, "BI's binarize.exe found in registry, but doesn't exist."));
    }

    let dependencies = if false { //input.extension().unwrap() == "p3d" {
        extract_dependencies(&input).prepend_error("Failed to read P3D:")?
    } else {
        Vec::new()
    };

    let mut input_dir = PathBuf::from(input.parent().unwrap());
    while !input_dir.join("config.cpp").exists() {
        if input_dir.parent().is_none() {
            return Err(Error::new(ErrorKind::Other, "Failed to find config.cpp."));
        }
        input_dir = PathBuf::from(input_dir.parent().unwrap());
    }
    let name = input.file_name().unwrap().to_str().unwrap().to_string();
    let input_tempdir = create_temp_directory(&name).prepend_error("Failed to create tempfolder:")?;
    let output_tempdir = create_temp_directory(&format!("{}.out", name)).prepend_error("Failed to create tempfolder:")?;

    copy(input, input_tempdir.join(&name)).prepend_error("Failed to copy file:")?;
    copy(input_dir.join("config.cpp"), input_tempdir.join("config.cpp")).prepend_error("Failed to copy config.cpp:")?;
    if input_dir.join("model.cfg").exists() {
        copy(input_dir.join("model.cfg"), input_tempdir.join("model.cfg")).prepend_error("Failed to copy model.cfg:")?;
    }

    for dep in dependencies {
        let mut include_path = if dep.chars().nth(0).unwrap() == '\\' {
            dep
        } else {
            format!("\\{}", dep).to_string()
        };

        match find_include_file(&include_path, Some(&input_dir), &includefolders) {
            Ok(real_path) => {
                let target = input_tempdir.join(PathBuf::from(include_path[1..].to_string()));
                create_dir_all(target.parent().unwrap()).prepend_error("Failed to copy dependency:")?;
                copy(real_path, target).prepend_error("Failed to copy dependency:")?;
            },
            Err(_msg) => {
                warning(format!("Failed to find dependency \"{}\".", include_path), Some("p3d-dependency-not-found"), (Some(input.to_str().unwrap().to_string()), None));
            }
        }
    }

    let piped = var("BIOUTPUT").unwrap_or("0".to_string()) == "1";

    let binarize_output = Command::new(binarize_exe)
        .args(&["-norecurse", "-always", "-silent", "-maxProcesses=0", input_tempdir.to_str().unwrap(), output_tempdir.to_str().unwrap()])
        .stdout(if piped { Stdio::inherit() } else { Stdio::null() })
        .stderr(if piped { Stdio::inherit() } else { Stdio::null() })
        .output().unwrap();

    if !binarize_output.status.success() {
        let msg = match binarize_output.status.code() {
            Some(code) => format!("binarize.exe terminated with exit code: {}", code),
            None => "binarize.exe terminated by signal.".to_string()
        };
        let outputhint = if var("BIOUTPUT").unwrap_or("0".to_string()) == "1" {
            "\nUse BIOUTPUT=1 to see binarize.exe's output."
        } else { "" };

        return Err(Error::new(ErrorKind::Other, format!("{}{}", msg, outputhint)));
    }

    let result_path = output_tempdir.join(name);
    let mut buffer: Vec<u8> = Vec::new();

    {
        let mut file = File::open(result_path).prepend_error("Failed to open binarize.exe output:")?;
        file.read_to_end(&mut buffer).prepend_error("Failed to read binarize.exe output:")?;
    }

    remove_dir_all(input_tempdir).prepend_error("Failed to remove temp directory:")?;
    remove_dir_all(output_tempdir).prepend_error("Failed to remove temp directory:")?;

    Ok(Cursor::new(buffer.into_boxed_slice()))
}

pub fn cmd_binarize(input: PathBuf, output: PathBuf, includefolders: &Vec<PathBuf>) -> Result<(), Error> {
    if !cfg!(windows) {
        return Err(Error::new(ErrorKind::Other, "binarize.exe is only available on windows. Use rapify to binarize configs."));
    }

    let cursor = binarize(&input, includefolders)?;
    let mut file = File::create(output).prepend_error("Failed to open output:")?;
    file.write_all(cursor.get_ref()).prepend_error("Failed to write result to file:")?;

    Ok(())
}
