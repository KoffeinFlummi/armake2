use std::io::{Error};
use std::fs::{File, create_dir_all, copy, remove_dir_all};
use std::collections::{HashSet};
use std::path::{PathBuf};
use std::env::temp_dir;
use std::process::Command;

#[cfg(windows)]
use winreg::RegKey;
#[cfg(windows)]
use winreg::enums::*;

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
    let p3d = P3D::read(&mut file)?;

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

    //println!("creating temp dir: {:?}", path);
    create_dir_all(&path)?;

    Ok(path)
}


pub fn cmd_binarize(input: PathBuf, output: PathBuf) -> i32 {
    if !cfg!(windows) {
        eprintln!("binarize.exe is only available on windows. use rapify to rapify configs.");
        return 1;
    }

    let binarize_exe = find_binarize_exe().expect("Failed to find BI's binarize.exe");
    if !binarize_exe.exists() {
        eprintln!("binarize.exe found, but doesn't exist");
        return 2;
    }

    let dependencies = if input.extension().unwrap() == "p3d" {
        extract_dependencies(&input).expect("Failed to read P3D")
    } else {
        Vec::new()
    };

    let mut search_paths: Vec<String> = Vec::new();
    search_paths.push(".".to_string());

    let input_dir = PathBuf::from(input.parent().unwrap());
    let name = input.file_name().unwrap().to_str().unwrap().to_string();
    let input_tempdir = create_temp_directory(&name).expect("Failed to create tempfolder");
    let output_tempdir = create_temp_directory(&format!("{}.out", name)).expect("Failed to create tempfolder");

    copy(input, input_tempdir.join(&name)).expect("Failed to copy file");
    if input_dir.join("config.cpp").exists() {
        copy(input_dir.join("config.cpp"), input_tempdir.join("config.cpp")).expect("Failed to copy config.cpp");
    }
    if input_dir.join("model.cfg").exists() {
        copy(input_dir.join("model.cfg"), input_tempdir.join("model.cfg")).expect("Failed to copy model.cfg");
    }

    for dep in dependencies {
        let mut include_path = if dep.chars().nth(0).unwrap() == '\\' {
            dep
        } else {
            format!("\\{}", dep).to_string()
        };

        match find_include_file(&include_path, Some(&input_dir), &search_paths) {
            Ok(real_path) => {
                let target = input_tempdir.join(PathBuf::from(include_path[1..].to_string()));
                create_dir_all(target.parent().unwrap()).expect("Failed to copy dependency");
                copy(real_path, target).expect("Failed to copy dependency");
            },
            Err(_msg) => {
                println!("Failed to find {}", include_path);
            }
        }
    }

    let binarize_output = Command::new(binarize_exe)
        .args(&["-norecurse", "-always", "-silent", "-maxProcesses=0", input_tempdir.to_str().unwrap(), output_tempdir.to_str().unwrap()])
        .output().unwrap();

    assert!(binarize_output.status.success());

    let result_path = output_tempdir.join(name);
    copy(result_path, output).expect("Failed to copy result");

    remove_dir_all(input_tempdir).expect("Failed to remove temp directory");
    remove_dir_all(output_tempdir).expect("Failed to remove temp directory");

    0
}
