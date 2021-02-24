extern crate base64;
extern crate bincode;
extern crate clap;
extern crate lzjd;
#[macro_use]
extern crate failure_derive;

mod crc32;
mod murmur3;

use murmur3::Murmur3BuildHasher;

use lzjd::{LZDict, LZJDError};

use std::fs::File;
use std::io::Write;
use std::io::{self, BufRead, BufReader, BufWriter, Read};
use std::path::Path;
use std::path::PathBuf;
use std::process;
use std::rc::Rc;

use clap::{App, Arg};
use rayon::prelude::*;
use walkdir::WalkDir;

#[derive(Debug, Fail)]
enum Error {
    #[fail(display = "IO error: {}", err)]
    Io {
        #[cause]
        err: io::Error,
    },
    #[fail(display = "Walkdir error: {}", err)]
    Walkdir {
        #[cause]
        err: walkdir::Error,
    },
    #[fail(display = "ThreadPoolBuild error: {}", err)]
    ThreadPoolBuild {
        #[cause]
        err: rayon::ThreadPoolBuildError,
    },
    #[fail(display = "{}", err)]
    LZJD {
        #[cause]
        err: LZJDError,
    },
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io { err }
    }
}

impl From<walkdir::Error> for Error {
    fn from(err: walkdir::Error) -> Self {
        Error::Walkdir { err }
    }
}

impl From<rayon::ThreadPoolBuildError> for Error {
    fn from(err: rayon::ThreadPoolBuildError) -> Self {
        Error::ThreadPoolBuild { err }
    }
}

impl From<LZJDError> for Error {
    fn from(err: LZJDError) -> Self {
        Error::LZJD { err }
    }
}

type Result<T> = std::result::Result<T, Error>;

fn main() {
    let cpus = &num_cpus::get().to_string();

    let matches = App::new("LZJD")
        .version("1.0")
        .author("Henk Dieter Oordt <henkdieter@tweedegolf.com>")
        .about("Calculates Lempel-Ziv Jaccard distance of input binaries. Based on jLZJD (https://github.com/EdwardRaff/jLZJD).")
        .arg(
            Arg::with_name("deep")
                .short("r")
                .long("deep")
                .help("generate SDBFs from directories and files")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("compare")
                .short("c")
                .long("compare")
                .help("compare SDBFs in file, or two SDBF files")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("gen-compare")
                .short("g")
                .long("gen-compare")
                .help("compare all pairs in source data")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("threshold")
                .short("t")
                .long("threshold")
                .help("only show results >= threshold")
                .takes_value(true)
                .default_value("1")
                .value_name("THRESHOLD"),
        )
        .arg(
            Arg::with_name("threads")
                .short("p")
                .long("--threads")
                .help("restrict compute threads to N threads")
                .takes_value(true)
                .default_value(cpus)
                .value_name("THREADS")
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .help("send output to files")
                .takes_value(true)
                .value_name("FILE"),
        )
        .arg(
            Arg::with_name("input")
                .help("Sets the input file to use")
                .value_name("INPUT")
                .required(true)
                .multiple(true),
        )
        .get_matches();
    if let Err(e) = run(matches) {
        eprintln!("{}", e);
        process::exit(-1);
    }
}

fn run(matches: clap::ArgMatches) -> Result<()> {
    let deep = matches.is_present("deep");
    let to_compare = matches.is_present("compare");
    let gen_compare = matches.is_present("gen-compare");

    let threshold = matches
        .value_of("threshold")
        .map(|t| t.parse::<u32>().ok())
        .unwrap_or(Some(1))
        .unwrap();

    let num_threads = matches
        .value_of("threads")
        .map(|p| p.parse::<usize>().ok())
        .unwrap_or(Some(4))
        .unwrap();

    let input_paths: Vec<PathBuf> = if deep {
        matches.args["input"]
            .vals
            .iter()
            .map(PathBuf::from)
            .flat_map(WalkDir::new)
            .try_fold(
                vec![],
                |mut v: Vec<PathBuf>, r: walkdir::Result<walkdir::DirEntry>| match r {
                    Ok(entry) => {
                        let path = entry.path();
                        if path.is_file() {
                            v.push(path.to_owned());
                        }
                        Ok(v)
                    }
                    Err(e) => Err(e),
                },
            )?
    } else {
        matches.args["input"]
            .vals
            .iter()
            .map(PathBuf::from)
            .collect()
    };

    let output_path = matches.value_of("output").map(PathBuf::from);

    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()?;

    let mut writer = create_out_writer(&output_path)?;

    if to_compare {
        if input_paths.is_empty() || input_paths.len() > 2 {
            return Err(LZJDError::from("Can only compare at most two indexes at a time!").into());
        }

        let hashes_a: Rc<Vec<(LZDict, String)>> = Rc::from(read_hashes_from_file(&input_paths[0])?);

        let hashes_b = if input_paths.len() == 2 {
            Rc::from(read_hashes_from_file(&input_paths[1])?)
        } else {
            Rc::clone(&hashes_a)
        };

        compare(&hashes_a, &hashes_b, threshold, &mut writer)?;
    } else if gen_compare {
        gen_comp(&input_paths, threshold, &mut writer)?;
    } else {
        hash_files(&input_paths, Some(&mut writer))?;
    }

    Ok(())
}

fn read_hashes_from_file(path: &Path) -> Result<Vec<(LZDict, String)>> {
    let file_handle = File::open(path)?;

    BufReader::new(file_handle)
        .lines()
        .try_fold(vec![], |mut v, line| {
            let line = line?;
            let line = line.trim();
            if !line.is_empty() {
                match line.rfind(':') {
                    Some(colon_index) if colon_index > 5 => {
                        let file_name = &line[5..colon_index];
                        let b64 = &line[colon_index + 1..];
                        let dict = LZDict::from_base64_string(b64)?;
                        v.push((dict, file_name.to_owned()));
                    }
                    _ => return Err(LZJDError::from("Could not parse line").into()),
                }
            }
            Ok(v)
        })
}

/// Perform comparisons of the given digests lists. If each list points to
/// the same object, only the above-diagonal elements of the comparison
/// matrix will be performed
fn compare(
    dicts_a: &[(LZDict, String)],
    dicts_b: &[(LZDict, String)],
    threshold: u32,
    writer: &mut dyn Write,
) -> Result<()> {
    let same = dicts_a as *const _ == dicts_b as *const _;
    let similarities: Vec<(String, String, u32)> = dicts_a
        .par_iter()
        .enumerate()
        .fold(
            || vec![],
            |mut v, (i, (dict_a, name_a))| {
                let j_start = if same { i + 1 } else { 0 };
                dicts_b.iter().skip(j_start).for_each(|(dict_b, name_b)| {
                    let similarity = (dict_a.similarity(dict_b) * 100.).round() as u32;
                    if similarity >= threshold {
                        v.push((name_a.to_owned(), name_b.to_owned(), similarity));
                    }
                });
                v
            },
        )
        .reduce(
            || vec![],
            |mut v, mut r| {
                v.append(&mut r);
                v
            },
        );

    similarities
        .iter()
        .try_for_each(|(name_a, name_b, similarity)| {
            writer.write_fmt(format_args!("{}|{}|{:03}\n", name_a, name_b, similarity))
        })?;

    Ok(())
}

/// Generate the set of digests and do the all pairs comparison at the same time.
fn gen_comp(paths: &[PathBuf], threshold: u32, writer: &mut dyn Write) -> Result<()> {
    let dicts: Rc<Vec<(LZDict, String)>> = Rc::from(hash_files(paths, None)?);

    compare(&dicts, &dicts, threshold, writer)
}

/// Digest and print out the hashes for the given list of files
fn hash_files(paths: &[PathBuf], writer: Option<&mut dyn Write>) -> Result<Vec<(LZDict, String)>> {
    let build_hasher = Murmur3BuildHasher;

    let dicts: Result<Vec<(LZDict, String)>> = paths
        .par_iter()
        .try_fold(
            || vec![],
            |mut v, r| {
                let file = File::open(r)?;

                let path_name = r.to_str().unwrap();

                let bytes = BufReader::new(file)
                    .bytes()
                    .map(std::result::Result::unwrap);

                v.push((
                    LZDict::from_bytes_stream(bytes, &build_hasher),
                    path_name.to_owned(),
                ));

                Ok(v)
            },
        )
        .try_reduce(
            || vec![],
            |mut v, mut results| {
                v.append(&mut results);
                Ok(v)
            },
        );
    let dicts = dicts?;
    if let Some(writer) = writer {
        dicts.iter().try_for_each(|d| {
            writer.write_fmt(format_args!("lzjd:{}:{}\n", d.1, d.0.to_string()))
        })?;
    }
    Ok(dicts)
}

fn create_out_writer(out_path: &Option<PathBuf>) -> Result<Box<dyn Write>> {
    if let Some(path) = out_path {
        Ok(Box::from(BufWriter::new(File::create(path)?)))
    } else {
        Ok(Box::from(BufWriter::new(io::stdout())))
    }
}
