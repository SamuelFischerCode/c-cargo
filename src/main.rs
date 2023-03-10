// Commit: #![forbid(warnings)] & #![deny(clippy::unwrap_used)]
// Dev: #![allow(warnings)]
#![deny(clippy::unwrap_used)]
#![forbid(warnings)]

use std::os::unix::ffi::OsStringExt;
use std::path::Path;
use std::{env, fs, io, process};
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

fn main() {
    match args_parse(1).to_ascii_lowercase().as_str() {
        "new" => new(args_parse(2)).unwrap_or_else(|e| {
            eprintln!("Error making new project \n\t{e:?}");
            process::exit(1);
        }),
        "update" => update().unwrap_or_else(|e| {
            eprintln!("Error updating project \n\t{e:?}");
            process::exit(1);
        }),
        _ => {
            eprintln!("Action not known");
            process::exit(1);
        }
    };
}

#[derive(Debug)]
enum Error {
    DirCreationIssue(io::Error),
    FileWritingIssue(io::Error),
    ProjNotInitialized,
    MakeTOMLNotRead(io::Error),
    MakeTOMLNotParsed(toml::de::Error),
    MakeTOMLCompilerFlagMissing,
    Other(String),
}

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

    let contents = include_str!("../template/Make.toml").to_owned();

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

        let compiler = match toml["compiler"].as_str() {
            Some(t) => t,
            None => return Err(Error::MakeTOMLCompilerFlagMissing),
        }
        .to_owned();

        let ret = gen_out(&compiler, &"src".to_owned())?;
        let mut out = ret.0;
        out.push_str("all : ");
        ret.1.iter().for_each(|x| {
            out.push_str(format!("{x} ").as_str());
        });
        out.push_str("\n\t");
        out.push_str(format!("{compiler} -o target/app.out ").as_str());
        ret.1.iter().for_each(|x| {
            out.push_str(format!("{x} ").as_str());
        });
        out.push_str("\n\n");

        out.push_str(
            "run : all \n\t\
        ./target/app.out",
        );

        // match fs::write("Makefile", out) {
        //     Err(e) => return Err(FileWritingIssue(e)),
        //     _ => {}
        // }
        if let Err(e) = fs::write("Makefile", out) {
            return Err(Error::FileWritingIssue(e));
        }

        Ok(())
    } else {
        Err(Error::ProjNotInitialized)
    }
}

fn gen_out(compiler: &String, path: &String) -> Result<(String, Vec<String>), Error> {
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
        dbg!(&file);

        let path = Path::new(&file);
        if path.is_dir() {
            let ret = gen_out(compiler, &file)?;
            push = ret.0;
            ret.1.into_iter().for_each(|x| {
                out_vec.push(x);
            });
        }

        if path.is_file() && &file[file.len() - 4..] == ".cpp" {
            let part_path = &file[4..file.len() - 4];
            push = format!("target/{part_path}.o : src/{part_path}.cpp \n\t{compiler} -c src/{part_path}.cpp -o target/{part_path}.o\n\n");
            out_vec.push(format!("target/{part_path}.o"));
        }

        out.push_str(push.as_str());
    }

    Ok((out, out_vec))
}
