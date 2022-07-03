use std::{fs::File, fs::ReadDir, path::Path, path::PathBuf};

use binread::{io::Cursor, BinRead};

use binwrite::BinWrite;

mod rdb;
use rdb::Rdb;

mod ktid;
use ktid::{KTID, ktid};

mod typeinfo;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "RdbTool",
    about = "Simple command-line tool to manipulate RDB files."
)]
struct Opt {
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    /// Patch a RDB file
    Patch(Patch),
    /// Output relevant informations about a RDB entry
    Print(Print),
}

#[derive(Debug, StructOpt)]
struct Patch {
    #[structopt(parse(from_os_str), help = "Path to the RDB file")]
    pub path: PathBuf,
    #[structopt(parse(from_os_str), help = "Output path to the RDB file")]
    pub out_path: PathBuf,
    #[structopt(parse(from_os_str), default_value = "patch", help = "Directory where the files to patch are located")]
    pub data_path: PathBuf,
}

#[derive(Debug, StructOpt)]
struct Print {
    #[structopt(parse(from_os_str), help = "Path to the RDB file")]
    pub path: PathBuf,
    #[structopt(help = "The KTID you would like to print")]
    pub ktid: String,
}

fn patch_rdb(args: &Patch) -> Result<(), String> {
    let mut rdb = Rdb::open(&args.path).unwrap();

    let external_path = if args.data_path.is_relative() {

        let rdb_dir = if args.path.is_relative() {
            std::fs::canonicalize(&args.path).unwrap().parent().unwrap().to_path_buf()
        } else {
            args.path.parent().unwrap().to_path_buf()
        };

        rdb_dir.join(&args.data_path)
    } else {
        args.data_path.to_path_buf()
    };

    if !external_path.exists() {
        return Err(format!("Couldn't find a directory to patch ('{}' was used). Consider making it?", external_path.display()));
    }

    let files = match std::fs::read_dir(external_path) {
        Ok(files) => files,
        Err(_) => return Err("How did you even managed to delete the directory this fast? Stop that.".to_string()),
    };

    for entry in files {
        let entry = entry.unwrap();
        let metadata = entry.metadata().unwrap();

        // We don't care about subdirectories
        if metadata.is_dir() {
            continue;
        }

        let path = &entry.path();

        // Check if we're dealing with a KTID or an actual filename
        let filename = if path.file_name().unwrap().to_str().unwrap().starts_with("0x") {
            // Strip the extension (Cethleann keeps the extension even if the hash is missing)
            path.file_stem().unwrap().to_str().unwrap()
        } else {
            // Get the full filename with extension
            path.file_name().unwrap().to_str().unwrap()
        };

        match rdb.get_entry_by_ktid_mut(crate::ktid(filename)) {
            Some(entry_found) => {
                println!("Patching {}", filename);
                entry_found.make_external();
                entry_found.make_uncompressed();
                entry_found.set_external_file(&entry.path());
            },
            None => println!("File {} not found in the RDB. Skipping.", filename),
        }
    }

    let mut bytes = vec![];
    rdb.write(&mut bytes).unwrap();

    std::fs::write(&args.out_path, bytes).unwrap();

    Ok(())
}

fn main() {
    let opt = Opt::from_args_safe().unwrap_or_else(|err| {
        println!("{}", err);
        std::process::exit(1);
    });

    match opt.cmd {
        Command::Patch(args) => {
            if let Err(error_msg) = patch_rdb(&args) {
                println!("{}", error_msg);
            }
        },
        Command::Print(args) => {
            let ktid = ktid(&args.ktid);
            let rdb = Rdb::read(&mut Cursor::new(&std::fs::read(&args.path).unwrap())).unwrap();
            let entry = rdb.get_entry_by_ktid(ktid).unwrap();
            println!("{:#?}", entry);
        },
    }
}

mod tests {
    use std::io::Write;

    use super::*;

    //const TEST_CONTENTS: &[u8] = include_bytes!("../system.rdb");

    #[test]
    fn test() {
        let test = typeinfo::object::sound::bank::ID;
    }

    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct TypeInfoEntry {
        pub typekind: String,
        ktid: String,
        pub typename: String,
    }

    #[test]
    fn generate_typeinfos_lmao_gross() {
        let mut rdr = csv::ReaderBuilder::new().has_headers(false).from_path("typeinfos.csv").unwrap();

        let typeinfos: Vec<String> = rdr.deserialize().into_iter().filter_map(|result| {
            let record: TypeInfoEntry = result.unwrap();

            //dbg!(record);
            if record.typekind == "TypeInfo" {
                Some(record.typename)
            } else {
                None
            }
        }).collect();

        for typeinfo in &typeinfos {
            let mut path = PathBuf::from(format!(".\\src\\{}", &typeinfo.replace("::", "\\")).to_lowercase());
            std::fs::create_dir_all(&path).unwrap();
            
            let mut mod_path = path.join("mod.rs");

            if !mod_path.exists() {
                let mut file = std::fs::OpenOptions::new().create(true).write(true).open(&dbg!(&mod_path)).unwrap();
                let stem = mod_path.parent().unwrap().file_stem().unwrap().to_str().unwrap();
                file.write_all(format!("use crate::ktid::KTID;\n\npub const ID: KTID = KTID({});", crate::ktid::ktid(&typeinfo).as_u32()).as_bytes()).unwrap();
            }
        }

        for typeinfo in &typeinfos {
            let mut path = PathBuf::from(format!(".\\src\\{}", &typeinfo.replace("::", "\\")));

            path.ancestors().for_each(|ancestor| {
                let dirs: Vec<String> = std::fs::read_dir(ancestor).unwrap().filter_map(|dir| {
                    let dir = dir.unwrap();
                    if dir.path().is_dir() {
                        Some(dir.file_name().to_str().unwrap().to_string())
                    } else {
                        None
                    }
                }).collect();

                let mod_path = ancestor.join("mod.rs");

                if !mod_path.exists() {
                    let mut output = String::new();

                for dir in dirs {
                    output.push_str(&format!("pub mod {};\n", dir));
                }

                std::fs::write(&mod_path, &output);
                }
            });
        }

        println!("{:x}", ktid::ktid("TypeInfo::Object::3D::Displayset::TrianglesEx").as_u32())
    }

    // #[test]
    // fn type_8_search() {
    //     let mut rdb: Rdb = Rdb::read(&mut Cursor::new(TEST_CONTENTS)).unwrap();
    //     //let entry = rdb.get_entry_by_KTID(0xf82a2296).unwrap();
    //     let entry = rdb.entries.iter().find(|lmao| lmao.entry_type != 0 && lmao.entry_type != 1 && lmao.entry_type != 4 && lmao.entry_type != 8 && lmao.entry_type != 12 && lmao.string_size != 0);
    //     dbg!(entry);
    // }

    #[test]
    fn patch_texternal() {
        //let mut rdb: Rdb = Rdb::read(&mut Cursor::new(TEST_CONTENTS)).unwrap();
        patch_rdb(&Opt { path: PathBuf::from("RRPreview.rdb"), out_path: PathBuf::from("RRPreview.rdb"), data_path: PathBuf::from("data") });
        //patch_rdb(Path::new("KIDSSystemResource.rdb"), Path::new("cock.rdb"));
        // let entry = rdb.get_entry_by_KTID(0x0a696242).unwrap();
        // entry.patch_external_file();
        //dbg!(entry);
    }
}
