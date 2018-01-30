extern crate cab;
extern crate clap;

use cab::{Cabinet, CabinetBuilder, CompressionType, FileEntry, FolderEntry};
use clap::{App, Arg, SubCommand};
use std::fs::{self, File};
use std::io;
use std::path::PathBuf;

// ========================================================================= //

fn main() {
    let matches = App::new("cabtool")
        .version("0.1")
        .author("Matthew D. Steele <mdsteele@alum.mit.edu>")
        .about("Manipulates CAB files")
        .subcommand(SubCommand::with_name("cat")
                        .about("Concatenates and prints streams")
                        .arg(Arg::with_name("cab").required(true))
                        .arg(Arg::with_name("file").multiple(true)))
        .subcommand(SubCommand::with_name("create")
                        .about("Creates a new cabinet")
                        .arg(Arg::with_name("compress")
                                 .takes_value(true)
                                 .value_name("TYPE")
                                 .short("c")
                                 .long("compress")
                                 .help("Sets compression type"))
                        .arg(Arg::with_name("output")
                                 .takes_value(true)
                                 .value_name("PATH")
                                 .short("o")
                                 .long("output")
                                 .help("Sets output path"))
                        .arg(Arg::with_name("file").multiple(true)))
        .subcommand(SubCommand::with_name("ls")
                        .about("Lists files in the cabinet")
                        .arg(Arg::with_name("long")
                                 .short("l")
                                 .help("Lists in long format"))
                        .arg(Arg::with_name("cab").required(true)))
        .get_matches();
    if let Some(submatches) = matches.subcommand_matches("cat") {
        let mut cabinet = open_cab(submatches.value_of("cab").unwrap())
            .unwrap();
        if let Some(filenames) = submatches.values_of("file") {
            for filename in filenames {
                let mut file_reader = cabinet.read_file(filename).unwrap();
                io::copy(&mut file_reader, &mut io::stdout()).unwrap();
            }
        }
    } else if let Some(submatches) = matches.subcommand_matches("create") {
        let ctype = match submatches.value_of("compress") {
            None => CompressionType::MsZip,
            Some("none") => CompressionType::None,
            Some("mszip") => CompressionType::MsZip,
            Some(value) => panic!("Invalid compression type: {}", value),
        };
        let out_path = if let Some(path) = submatches.value_of("output") {
            PathBuf::from(path)
        } else {
            let mut path = PathBuf::from("out.cab");
            let mut index: i32 = 0;
            while path.exists() {
                index += 1;
                path = PathBuf::from(format!("out{}.cab", index));
            }
            path
        };
        let mut builder = CabinetBuilder::new();
        {
            let folder = builder.add_folder(ctype);
            if let Some(filenames) = submatches.values_of("file") {
                for filename in filenames {
                    folder.add_file(filename.to_string());
                }
            }
        }
        let file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&out_path)
            .unwrap();
        let mut cabinet = builder.build(file).unwrap();
        while let Some(mut writer) = cabinet.next_file().unwrap() {
            let mut file = File::open(writer.file_name()).unwrap();
            io::copy(&mut file, &mut writer).unwrap();
        }
        cabinet.finish().unwrap();
    } else if let Some(submatches) = matches.subcommand_matches("ls") {
        let long = submatches.is_present("long");
        let cabinet = open_cab(submatches.value_of("cab").unwrap()).unwrap();
        for (index, folder) in cabinet.folder_entries().enumerate() {
            for file in folder.file_entries() {
                list_file(index, folder, file, long);
            }
        }
    }
}

// ========================================================================= //

fn list_file(folder_index: usize, folder: &FolderEntry, file: &FileEntry,
             long: bool) {
    if !long {
        println!("{}", file.name());
        return;
    }
    let ctype = match folder.compression_type() {
        CompressionType::None => "None".to_string(),
        CompressionType::MsZip => "MsZip".to_string(),
        CompressionType::Quantum(v, m) => format!("Q{}/{}", v, m),
        CompressionType::Lzx(w) => format!("Lzx{}", w),
    };
    let file_size = if file.uncompressed_size() >= 100_000_000 {
        format!("{} MB", file.uncompressed_size() / (1 << 20))
    } else if file.uncompressed_size() >= 1_000_000 {
        format!("{} kB", file.uncompressed_size() / (1 << 10))
    } else {
        format!("{} B ", file.uncompressed_size())
    };
    println!("{}{}{}{}{}{} {:>2} {:<5} {:>10} {} {}",
             if file.is_read_only() { 'R' } else { '-' },
             if file.is_hidden() { 'H' } else { '-' },
             if file.is_system() { 'S' } else { '-' },
             if file.is_archive() { 'A' } else { '-' },
             if file.is_exec() { 'E' } else { '-' },
             if file.is_name_utf() { 'U' } else { '-' },
             folder_index,
             ctype,
             file_size,
             file.datetime(),
             file.name());
}

fn open_cab(path: &str) -> io::Result<Cabinet<File>> {
    Cabinet::new(File::open(path)?)
}

// ========================================================================= //
