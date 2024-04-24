#![feature(coroutines)]
#![feature(iter_from_coroutine)]

use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::{env, fs};

use anyhow::{Ok, Result};
use clap::{Arg, ArgAction, Command};

const CR: u8 = 0x0D;
const LF: u8 = 0x0A;

fn cli() -> Command {
    Command::new(clap::crate_name!())
        .version(clap::crate_version!())
        .about(clap::crate_description!())
        .author(clap::crate_authors!())
        .args_conflicts_with_subcommands(true)
        .arg_required_else_help(true)
        .arg(
            Arg::new("include")
                .help("Include filepaths with a pattern. (appending)")
                .value_name("PATTERN")
                .num_args(1..)
                .required(true)
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("exclude")
                .short('e')
                .long("exclude")
                .help("Exclude filepaths with a pattern. (appending)")
                .value_name("PATTERN")
                .num_args(1..)
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("eol")
                .short('l')
                .long("eol")
                .help("Set line ending sequence to convert to.")
                .value_name("EOL")
                .value_parser(["LF", "CRLF", "CR"])
                .default_value("LF")
                .ignore_case(true)
                .global(true),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .help("Output directory of converted files, otherwise replace original files.")
                .value_name("DIR")
                .num_args(1),
        )
        .arg(
            Arg::new("case-sensitive")
                .short('c')
                .long("case-sensitive")
                .help("Use case sensitive matching in patterns (on Windows).")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("dry-run")
                .short('d')
                .long("dry-run")
                .help("Print filepaths that would be affected, without modifying files.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Print out debug information to stderr.")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .subcommand(
            Command::new("stdin")
                .about("Read stdin as input, write to the specified file.")
                .arg(
                    Arg::new("file")
                        .help("Output filepath.")
                        .value_name("FILE")
                        .num_args(1)
                        .required(true),
                )
                .arg(
                    Arg::new("stdout")
                        .short('p')
                        .long("stdout")
                        .help("Output to stdout. NOTE: Shell might force native EOL sequence!")
                        .conflicts_with("file")
                        .action(ArgAction::SetTrue),
                ),
        )
        .after_help("Exclusions take precedence over inclusions.")
}

fn exit_with_error(msg: impl std::fmt::Display) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

/// Read stdin input and write to `w` with the set end-of-line sequence.
fn stdin_to_output(mut w: impl Write, eol: Eol) -> Result<()> {
    // NOTE: Windows stdin impl only supports UTF-8.
    // TODO: Use byte transformer instead of lines iter.
    let stdin = std::io::stdin().lock();
    let mut lines = stdin.lines().peekable();
    while let Some(line) = lines.next() {
        let line = line.unwrap_or_else(|e| exit_with_error(e));
        let eol = if lines.peek().is_some() {
            eol.sequence()
        } else {
            ""
        };
        write!(w, "{line}{eol}")?;
    }
    Ok(())
}

/// Apply a conversion to a file, this assumes that path is an accessible file.
fn process_file(path: &Path, eol: Eol) -> Result<()> {
    debug_assert!(path.is_file());
    let input = File::open(path)?;
    let input = BufReader::new(input);
    let input = input
        .bytes()
        .map(|r| r.unwrap_or_else(|e| exit_with_error(e)));
    let temp = temp_file::empty();
    let output = OpenOptions::new().write(true).open(temp.path())?;
    let mut output = BufWriter::new(output);
    let transform = eol.transform_fn();
    transform(input, &mut output)?;
    output.flush()?;
    fs::copy(temp.path(), path)?;
    Ok(())
}

// TODO: Use a logger for verbose.
fn main() -> Result<()> {
    let matches = cli().get_matches();
    let verbose = matches.get_flag("verbose");
    let eol: Eol = matches
        .get_one::<String>("eol")
        .unwrap_or_else(|| exit_with_error("Missing end-of-line sequence"))
        .parse()
        .unwrap_or_else(|e| exit_with_error(e));

    if verbose {
        eprintln!("Target sequence: {eol}");
    }

    // Subcommands:
    if let Some(sub_matches) = matches.subcommand_matches("stdin") {
        if let Some(output) = sub_matches.get_one::<String>("file") {
            let output = std::path::PathBuf::from(output);
            if !output.is_file() {
                exit_with_error("Output path must be a file.")
            };
            if verbose {
                eprintln!("Output: {}", output.display());
            }
            let file = fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(output)
                .unwrap_or_else(|e| exit_with_error(e));
            stdin_to_output(file, eol).unwrap_or_else(|e| exit_with_error(e));
        } else {
            if verbose {
                eprintln!("Output: stdout");
            }
            let stdout = std::io::stdout().lock();
            stdin_to_output(stdout, eol).unwrap_or_else(|e| exit_with_error(e));
        };

        return Ok(());
    }

    // Base command:
    let dry_run = matches.get_flag("dry-run");
    let glob_options = glob::MatchOptions {
        case_sensitive: matches.get_flag("case-sensitive"),
        ..Default::default()
    };

    if matches.get_many::<String>("exclude").is_some() {
        todo!("wip"); // TODO: To be implemented.
    }

    if matches.get_one::<String>("output").is_some() {
        todo!("wip"); // TODO: To be implemented.
    }

    let patterns = std::iter::from_coroutine(|| {
        if let Some(values) = matches.get_many::<String>("include") {
            for pat in values {
                yield glob::glob_with(pat, glob_options);
            }
        } else {
            eprintln!("No included files.");
        }
    });

    // This ensures that glob patterns are correct before doing any work.
    let paths = patterns
        .flat_map(|p| p.unwrap_or_else(|e| exit_with_error(e)))
        .map(|p| p.unwrap_or_else(|e| exit_with_error(e)))
        .filter(|p| p.is_file())
        .collect::<Vec<_>>();

    if verbose {
        eprintln!("Dry-run: {dry_run}");
        eprintln!("Case-sensitive: {}", glob_options.case_sensitive);
    }

    let mut stdout = std::io::stdout().lock();
    for path in paths {
        if dry_run {
            writeln!(stdout, "{}", path.display())?;
            continue;
        }
        if verbose {
            eprintln!("{}", path.display());
        }
        process_file(&path, eol)?;
    }

    Ok(())
}

/// End-of-line sequence.
#[derive(Debug, Clone, Copy)]
enum Eol {
    Lf,
    Crlf,
    Cr,
}

impl Eol {
    fn sequence(&self) -> &str {
        match self {
            Eol::Lf => "\n",
            Eol::Crlf => "\r\n",
            Eol::Cr => "\r",
        }
    }

    fn transform_fn<B: Iterator<Item = u8>, W: Write>(&self) -> fn(B, &mut W) -> Result<()> {
        fn convert(
            bytes: impl Iterator<Item = u8>,
            mut writer: impl Write,
            target: &[u8],
        ) -> Result<()> {
            let mut iter = bytes.peekable();
            while let Some(byte) = iter.next() {
                if byte == LF {
                    writer.write_all(target)?;
                } else if byte == CR {
                    _ = iter.next_if(|&n| n == LF);
                    writer.write_all(target)?;
                } else {
                    writer.write_all(&[byte])?;
                }
            }
            Ok(())
        }
        match self {
            Eol::Lf => |b, w| convert(b, w, &[LF]),
            Eol::Crlf => |b, w| convert(b, w, &[CR, LF]),
            Eol::Cr => |b, w| convert(b, w, &[CR]),
        }
    }
}

impl std::str::FromStr for Eol {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "lf" => Ok(Eol::Lf),
            "crlf" => Ok(Eol::Crlf),
            "cr" => Ok(Eol::Cr),
            _ => anyhow::bail!("Unknown end-of-line sequence"),
        }
    }
}

impl std::fmt::Display for Eol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Eol::Lf => "LF",
            Eol::Crlf => "CRLF",
            Eol::Cr => "CR",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test(eol: Eol, input: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let t = eol.transform_fn();
        t(input.bytes().map(|r| r.unwrap()), &mut out).unwrap();
        out
    }

    #[test]
    fn transform_lf() {
        assert_eq!(test(Eol::Lf, b""), b"");
        assert_eq!(test(Eol::Lf, b"abc"), b"abc");
        assert_eq!(test(Eol::Lf, b"\n"), b"\n");
        assert_eq!(test(Eol::Lf, b"\r\n"), b"\n");
        assert_eq!(test(Eol::Lf, b"x\rx\n"), b"x\nx\n");
    }

    #[test]
    fn transform_crlf() {
        assert_eq!(test(Eol::Crlf, b""), b"");
        assert_eq!(test(Eol::Crlf, b"abc"), b"abc");
        assert_eq!(test(Eol::Crlf, b"\n"), b"\r\n");
        assert_eq!(test(Eol::Crlf, b"\r\n"), b"\r\n");
        assert_eq!(test(Eol::Crlf, b"x\rx\n"), b"x\r\nx\r\n");
    }

    #[test]
    fn transform_cr() {
        assert_eq!(test(Eol::Cr, b""), b"");
        assert_eq!(test(Eol::Cr, b"abc"), b"abc");
        assert_eq!(test(Eol::Cr, b"\n"), b"\r");
        assert_eq!(test(Eol::Cr, b"\r\n"), b"\r");
        assert_eq!(test(Eol::Cr, b"x\rx\n"), b"x\rx\r");
    }
}
