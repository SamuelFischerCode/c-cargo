// Commit: #![forbid(warnings)] & #![deny(clippy::unwrap_used)]
// Dev: #![allow(warnings)]
#![forbid(warnings)]
#![deny(clippy::unwrap_used)]

#[cfg(target_family = "unix")]
use std::os::unix::ffi::OsStringExt;

use std::fmt;
use std::path::Path;
use std::{env, error, fs, io, process};
use toml::Table;

fn args_parse(i: usize) -> String {
    match env::args().nth(i) {
        Some(t) => t,
        None => {
            eprintln!("Not enough arguments \n\tat least {i} required");
            process::exit(1);
        }
    }
}

#[cfg(not(target_family = "windows"))]
fn main() {
    match args_parse(1).to_ascii_lowercase().as_str() {
        "new" => new(args_parse(2)).unwrap_or_else(|e| {
            eprintln!("Error making new project \n\t{e}");
            process::exit(1);
        }),
        "update" => update().unwrap_or_else(|e| {
            eprintln!("Error updating project \n\t{e}");
            process::exit(1);
        }),
        _ => {
            eprintln!("Action not known");
            process::exit(1);
        }
    };
}

#[cfg(target_family = "windows")]
fn main() {
    eprintln!("Windows doesn't work out of the box because the lack of the OsStringExt::into_vec() method");
}

#[derive(Debug)]
enum Error {
    DirCreationIssue(io::Error),
    FileWritingIssue(io::Error),
    ProjNotInitialized,
    MakeTOMLNotRead(io::Error),
    MakeTOMLNotParsed(toml::de::Error),
    MakeTOMLNameMissing,
    Other(String),
}

impl fmt::Display for Error {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Error::DirCreationIssue(int) => format!("Error creating directory \n\t\t{int}"),
                Error::FileWritingIssue(int) => format!("Error writing to a file \n\t\t{int}"),
                Error::ProjNotInitialized =>
                    "Project is not initialized \n\t\trun \"c-cargo new {proj_name}\"".to_owned(),
                Error::MakeTOMLNotRead(int) => format!("Error reading Make.toml \n\t\t{int}"),
                Error::MakeTOMLNotParsed(int) => format!("Error parsing Make.toml \n\t\t{int}"),
                Error::MakeTOMLNameMissing => "Add a name to your Make.toml".to_owned(),
                Error::Other(int) => format!("Unknown error \n\t\t{int}"),
            }
        )
    }
}

impl error::Error for Error {}

fn new(proj_name: String) -> Result<(), Error> {
    if let Err(e) = fs::create_dir(&proj_name) {
        return Err(Error::DirCreationIssue(e));
    }

    if let Err(e) = fs::create_dir(format!("{}/src", proj_name)) {
        return Err(Error::DirCreationIssue(e));
    }

    if let Err(e) = fs::create_dir(format!("{}/target", proj_name)) {
        return Err(Error::DirCreationIssue(e));
    }

    let contents = include_str!("../template/main.cpp").to_owned();

    if let Err(e) = fs::write(format!("{}/src/main.cpp", proj_name), contents) {
        return Err(Error::FileWritingIssue(e));
    }

    let contents = format!(r#"name = "{proj_name}""#);

    if let Err(e) = fs::write(format!("{}/Make.toml", proj_name), contents) {
        return Err(Error::FileWritingIssue(e));
    }

    println!("Done just run \n\t\"cd {proj_name}\" \n\t\"c-cargo update\"");

    Ok(())
}

fn update() -> Result<(), Error> {
    if Path::new("Make.toml").exists() {
        let toml = match match fs::read_to_string("Make.toml") {
            Ok(t) => t,
            Err(e) => return Err(Error::MakeTOMLNotRead(e)),
        }
        .parse::<Table>()
        {
            Ok(t) => t,
            Err(e) => return Err(Error::MakeTOMLNotParsed(e)),
        };

        let compiler = match toml.get("compiler") {
            Some(t) => t.as_str().unwrap_or("clang++"),
            None => "clang++",
        }
        .to_owned();
        let linker = match toml.get("linker") {
            Some(t) => t.as_str().unwrap_or(compiler.as_str()),
            None => compiler.as_str(),
        }
        .to_owned();
        let c_flags = match toml.get("c_flags") {
            Some(t) => t.as_str().unwrap_or(""),
            None => "",
        }
        .to_owned();
        let l_flags = match toml.get("l_flags") {
            Some(t) => t.as_str().unwrap_or(""),
            None => "",
        }
        .to_owned();
        let run_args = match toml.get("run_args") {
            Some(t) => t.as_str().unwrap_or(""),
            None => "",
        }
        .to_owned();
        let file_ext = match toml.get("file_ext") {
            Some(t) => t.as_str().unwrap_or(".cpp"),
            None => ".cpp",
        }
        .to_owned();
        let name = match toml.get("name") {
            Some(t) => match t.as_str() {
                Some(t) => t,
                None => return Err(Error::MakeTOMLNameMissing),
            },
            None => return Err(Error::MakeTOMLNameMissing),
        }
        .to_owned();

        let ret = gen_out(&compiler, &"src".to_owned(), &c_flags, &file_ext)?;
        let out = format!("clean : \n\t rm -rf target/*.o target/*.out\n\n{}all : {map}\n\t{linker} -o target/{name}.out {l_flags} {map}\n\nrun : all \n\t./target/{name}.out {run_args}", ret.0.as_str(),
        map = ret.1.iter().map(|x| {
            format!("{x} ")
        }).collect::<String>());

        if let Err(e) = fs::write("Makefile", out) {
            return Err(Error::FileWritingIssue(e));
        }

        Ok(())
    } else {
        Err(Error::ProjNotInitialized)
    }
}

fn gen_out(
    compiler: &String,
    path: &String,
    c_flags: &String,
    file_ext: &String,
) -> Result<(String, Vec<String>), Error> {
    let mut out = String::new();
    let mut out_vec = Vec::new();

    for item in match fs::read_dir(path) {
        Ok(t) => t,
        Err(e) => return Err(Error::Other(e.to_string())),
    } {
        let mut push = "".to_owned();

        let item = match item {
            Ok(t) => t,
            Err(e) => return Err(Error::Other(e.to_string())),
        };

        let file = match String::from_utf8(item.path().into_os_string().into_vec()) {
            Ok(t) => t,
            Err(e) => return Err(Error::Other(e.to_string())),
        };

        let path = Path::new(&file);
        if path.is_dir() {
            let ret = gen_out(compiler, &file, c_flags, file_ext)?;
            push = ret.0;
            ret.1.into_iter().for_each(|x| {
                out_vec.push(x);
            });
        }

        if path.is_file() && &file[file.len() - file_ext.len()..] == file_ext {
            let part_path = &file[4..file.len() - file_ext.len()];
            push = format!("target/{part_path}.o : src/{part_path}{file_ext} \n\t{compiler} {c_flags} -c src/{part_path}{file_ext} -o target/{part_path}.o\n\n");
            out_vec.push(format!("target/{part_path}.o"));
        }

        out.push_str(push.as_str());
    }

    Ok((out, out_vec))
}
