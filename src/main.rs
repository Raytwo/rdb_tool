use std::{fs::File, fs::ReadDir, path::Path, path::PathBuf};

use binread::{io::Cursor, BinRead};

use binwrite::BinWrite;

mod rdb;
use rdb::Rdb;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "RdbTool",
    about = "Simple command-line tool to manipulate RDB files."
)]
struct Opt {
    #[structopt(parse(from_os_str), help = "Path to the RDB file")]
    pub path: PathBuf,
    #[structopt(parse(from_os_str), help = "Output path to the RDB file")]
    pub out_path: PathBuf,
    #[structopt(parse(from_os_str), default_value = "data", help = "Directory where the files to patch are located")]
    pub data_path: PathBuf,
}

fn patch_rdb(args: &Opt) -> Result<(), String> {
    let mut rdb: Rdb = Rdb::read(&mut Cursor::new(&std::fs::read(&args.path).unwrap())).unwrap();

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

        let filename = &entry.file_name().to_str().unwrap().to_owned();

        if !filename.ends_with(".file") {
            println!("File {} does not have the '.file' extension. Skipping.", filename);
            continue;
        }

        if !filename.starts_with("0x") {
            println!("File {} is not named after an offset (Ex.: 0x69696969.file). Skipping", filename);
            continue;
        }

        match rdb.entries.iter_mut().find(|x| &x.get_external_path().to_str().unwrap().to_owned() == filename)  {
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
    println!("{:#?}", opt);

    if let Err(error_msg) = patch_rdb(&opt) {
        println!("{}", error_msg);
    }
}

mod tests {
    use super::*;

    const TEST_CONTENTS: &[u8] = include_bytes!("../ScreenLayout.rdb");

    #[test]
    fn test() {
        
    }

    #[test]
    fn type_8_search() {
        let mut rdb: Rdb = Rdb::read(&mut Cursor::new(TEST_CONTENTS)).unwrap();
        let entry = rdb.get_entry_by_KTID(0xf82a2296).unwrap();
        dbg!(entry);
    }

    #[test]
    fn patch_texternal() {
        //let mut rdb: Rdb = Rdb::read(&mut Cursor::new(TEST_CONTENTS)).unwrap();
        patch_rdb(Path::new("ScreenLayout.rdb"), Path::new("cock.rdb"));
        // let entry = rdb.get_entry_by_KTID(0x0a696242).unwrap();
        // entry.patch_external_file();
        //dbg!(entry);
    }
}
