# WIP

# newl – A tool for converting end-of-line sequences

`newl` is a CLI tool that can be used to change line endings in files.<br>

- It takes filepaths as a default input and you may also exclude paths.<br>
- It understands glob patterns for ease of use on Windows.
  _(On other operating systems the shell may do glob expansions.)_
- It can also use bytes from standard input stream and write the converted data to a file.

# Help

```
Usage: newl.exe [OPTIONS] <PATTERN>...
       newl.exe <COMMAND>

Commands:
  stdin  Read stdin as input, write to the specified file.
  help   Print this message or the help of the given subcommand(s)

Arguments:
  <PATTERN>...  Include filepaths with a pattern. (appending)

Options:
  -e, --exclude <PATTERN>...  Exclude filepaths with a pattern. (appending)
  -l, --eol <EOL>             Set line ending sequence to convert to. [default: LF] [possible values: LF, CRLF, CR]
  -o, --output <DIR>          Output directory of converted files, otherwise replace original files.
  -c, --case-sensitive        Use case sensitive matching in patterns (on Windows).
  -n, --dry-run               Print filepaths that would be affected, without modifying files.
  -d, --debug                 Print output bytes as debug representation to stdout.
  -v, --verbose               Print out debug information to stderr.
  -h, --help                  Print help
  -V, --version               Print version

Exclusions take precedence over inclusions.
```

```
Usage: newl.exe stdin [OPTIONS] [FILE]

Arguments:
  [FILE]  Output filepath.

Options:
  -p, --stdout     Output to stdout. NOTE: Shell might force native EOL sequence!
  -l, --eol <EOL>  Set line ending sequence to convert to. [default: LF] [possible values: LF, CRLF, CR]
  -d, --debug      Print output bytes as debug representation to stdout.
  -v, --verbose    Print out debug information to stderr.
  -h, --help       Print help
```
