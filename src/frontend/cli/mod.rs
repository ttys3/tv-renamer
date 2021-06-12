mod man;
use backend::{self, Arguments, Season, ScanDir, TargetErr};
use backend::tokenizer;
use self::man::MAN_PAGE;
use std::env;
use std::io::{self, Write, Read};
use std::fs;
use std::path::Path;
use std::process;
use tvdb;
use backend::{DRY_RUN, VERBOSE};

const EP_NO_VAL: &'static str = "no value was set for the episode count.\n";
const SR_NO_VAL: &'static str = "no value was set for the series name.\n";
const SN_NO_VAL: &'static str = "no value was set for the season number.\n";
const PD_NO_VAL: &'static str = "no value was set for the pad length.\n";
const TMP_NO_VAL:&'static str = "no value was set for the template.\n";

pub fn interface<A: Iterator<Item = String>>(args: A) {
    let stderr = &mut io::stderr();

    // Default CLI arguments
    let mut arguments = Arguments {
        flags:          0,
        season_index:   1,
        episode_index:  1,
        pad_length:     2,
        base_directory: String::with_capacity(256),
        series_name:    String::with_capacity(64),
        template:       tokenizer::default_template(),
    };

    // Attempt to parse the input arguments and act upon any errors that are returned
    if let Err(why) = parse_arguments(&mut arguments, args) {
        let _ = stderr.write(b"tv-renamer: ");
        match why {
            ParseError::NoEpisodeIndex           => { let _ = stderr.write(EP_NO_VAL.as_bytes()); },
            ParseError::NoSeriesIndex            => { let _ = stderr.write(SR_NO_VAL.as_bytes()); },
            ParseError::NoSeriesName             => { let _ = stderr.write(SN_NO_VAL.as_bytes()); },
            ParseError::NoTemplate               => { let _ = stderr.write(TMP_NO_VAL.as_bytes()); },
            ParseError::NoPadLength              => { let _ = stderr.write(PD_NO_VAL.as_bytes()); },
            ParseError::EpisodeIndexIsNaN(value) => { let _ = write!(stderr, "episode index, `{}`, is not a number\n", value); },
            ParseError::SeriesIndexIsNaN(value)  => { let _ = write!(stderr, "series index, `{}`, is not a number\n", value); },
            ParseError::PadLengthIsNaN(value)    => { let _ = write!(stderr, "pad length, `{}`, is not a number\n", value); },
            ParseError::InvalidArgument(value)   => { let _ = write!(stderr, "invalid argument: `{}`\n", value); },
            ParseError::TooManyArguments(value)  => { let _ = write!(stderr, "too many arguments: `{}`\n", value); }
            ParseError::NoCWD                    => { let _ = stderr.write(b"unable to get current working directory\n"); },
            ParseError::CWDNotValid              => { let _ = stderr.write(b"current working directory is not valid UTF-8\n"); }
        }
        process::exit(1);
    }

    // Collect a list of episodes within a directory and rename them.
    match backend::scan_directory(&arguments.base_directory, arguments.season_index) {
        // If the directory contains episodes, rename the episodes.
        Ok(ScanDir::Episodes(season)) => rename_season(stderr, &season, &arguments, arguments.episode_index),
        // If the directory contains seasons, rename the episodes in each season.
        Ok(ScanDir::Seasons(seasons)) => for season in seasons { rename_season(stderr, &season, &arguments, 1); },
        // If an error occurred, print an error and exit.
        Err(why) => {
            let _ = writeln!(stderr, "tv-renamer: {}", why);
            process::exit(1);
        }
    }
}

/// Renames all of the episodes in given season
fn rename_season(stderr: &mut io::Stderr, season: &Season, arguments: &Arguments, episode_no: u16) {
    let stdout = &mut io::stdout();
    let mut episode_no = episode_no;

    // TVDB
    // https://github.com/thetvdb/v4-api
    let api = tvdb::Tvdb::new("0629B785CE550C8D");
    let series_id = match api.search(Some(&arguments.series_name), Some("en")) {
        Ok(result) => result.data.unwrap()[0].id.unwrap(),
        Err(err)     => {
            let _ = write!(stderr, "tv-renamer: invalid TV series: {} err={}\n", &arguments.series_name, err);
            process::exit(1);
        }
    };

    for source in &season.episodes {
        match backend::collect_target(source, season.season_no, episode_no, arguments, &api, series_id) {
            Ok(target) => {
                // If the target exists, do not overwrite the target without first asking if it is OK.
                if target.exists() {
                    let stderr = io::stderr();
                    let mut stderr = stderr.lock();
                    let _ = writeln!(stderr, "tv-renamer: episode to be renamed already exists:\n{:?}\nIs it okay to overwrite? (y/n)", &target);
                    let mut input = [b'n'; 1];

                    if let Err(why) = io::stdin().read_exact(&mut input) {
                        let _ = writeln!(stderr, "tv-renamer: error reading from standard input: {:?}", why);
                        process::exit(1);
                    }

                    if input[0] != b'y' {
                        let _ = stderr.write_all(b"tv-renamer: stopping the renaming process.\n");
                        process::exit(1);
                    }
                }

                // If dry run or verbose is enabled, print the action being taken
                if arguments.flags & (DRY_RUN + VERBOSE) != 0 {
                    let _ = stdout.write(b"\x1b[1m\x1b[32m");
                    let _ = write!(stdout, "{:?}", backend::shorten_path(&source));
                    let _ = stdout.write(b"\x1b[0m -> ");
                    let _ = stdout.write(b"\x1b[1m\x1b[32m");
                    let _ = write!(stdout, "{:?}", backend::shorten_path(&target));
                    let _ = stdout.write(b"\x1b[0m\n");
                }

                // If dry run is not enabled, rename the file
                if arguments.flags & DRY_RUN == 0 {
                    if let Err(cause) = fs::rename(&source, &target) {
                        let _ = writeln!(stderr, "tv-renamer: rename failed: {:?}", cause.to_string());
                        process::exit(1);
                    }
                }

            },
            Err(why) => {
                let _ = stderr.write(b"tv-renamer: ");
                match why {
                    // The episode number was unable to be found in the TV series.
                    TargetErr::EpisodeDoesNotExist => {
                        let _ = writeln!(stderr, "unable to find episode {}", episode_no);
                    },
                    TargetErr::Extension => {
                        let _ = writeln!(stderr, "unable to get extension");
                    },
                    TargetErr::Parent => {
                        let _ = writeln!(stderr, "unable to get parent filepath");
                    }
                }
                process::exit(1);
            }
        }
        episode_no += 1;
    }
}

enum ParseError {
    NoEpisodeIndex,
    NoSeriesIndex,
    NoSeriesName,
    NoTemplate,
    NoPadLength,
    EpisodeIndexIsNaN(String),
    SeriesIndexIsNaN(String),
    PadLengthIsNaN(String),
    InvalidArgument(String),
    TooManyArguments(String),
    NoCWD,
    CWDNotValid,
}

/// Parse command-line arguments and update the `arguments` structure accordingly.
fn parse_arguments <A: Iterator<Item = String>> (
    arguments: &mut Arguments,
    mut iterator: A
) -> Result<(), ParseError> {
    while let Some(argument) = iterator.next() {
        if argument.starts_with('-') {
            match argument.as_str() {
                "-h" | "--help" => {
                    println!("{}", MAN_PAGE);
                    process::exit(0);
                }
                "-d" | "--dry-run" => arguments.flags |= DRY_RUN,
                "-e" | "--episode-start" => {
                    let value = iterator.next().ok_or(ParseError::NoEpisodeIndex)?;
                    arguments.episode_index = value.parse::<u16>()
                        .map_err(|_| ParseError::EpisodeIndexIsNaN(value))?;
                },
                "-n" | "--series-name" => {
                    arguments.series_name.push_str(&iterator.next().ok_or(ParseError::NoSeriesName)?);
                },
                "-s" | "--season-number" => {
                    let value = iterator.next().ok_or(ParseError::NoSeriesIndex)?;
                    arguments.season_index = value.parse::<u8>()
                        .map_err(|_| ParseError::SeriesIndexIsNaN(value))?;
                },
                "-t" | "--template" => {
                    let value = iterator.next().ok_or(ParseError::NoTemplate)?;
                    arguments.template = tokenizer::tokenize_template(&value);
                },
                "-p" | "--pad-length" => {
                    let value = iterator.next().ok_or(ParseError::NoPadLength)?;
                    arguments.pad_length = value.parse::<u8>()
                        .map_err(|_| ParseError::PadLengthIsNaN(value))?;
                },
                "-v" | "--verbose" => arguments.flags |= VERBOSE,
                _ => return Err(ParseError::InvalidArgument(argument))
            }
        } else if arguments.base_directory.is_empty() {
            arguments.base_directory = argument;
        } else {
            return Err(ParseError::TooManyArguments(argument));
        }
    }

    // Set to current working directory if no directory argument is given.
    if arguments.base_directory.is_empty() {
        let directory = env::current_dir().map_err(|_| ParseError::NoCWD)?;
        arguments.base_directory = directory.to_str().ok_or(ParseError::CWDNotValid)?.to_owned();
    }

    // If no series name was given, set the series name to the base directory
    if arguments.series_name.is_empty() {
        arguments.series_name = String::from(Path::new(&arguments.base_directory)
            .file_name().unwrap().to_str().unwrap());
    }

    Ok(())
}
