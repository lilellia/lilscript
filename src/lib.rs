use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use env_logger;
use log::debug;
use std::{error::Error, fs, path::PathBuf};

use crate::{md_handler::ToMarkdown, script::Script, tex_handler::Tex};

/// A module which handles the creation of `Script` objects and their components.
pub mod script;

/// A module which handles `Script` ⟷ TeX format inter-conversions
pub mod tex_handler;

/// A module which handles `Script` ⟷ Markdown format inter-conversions
pub mod md_handler;
// use crate::md_handler::ToMarkdown;

/// For command-line parsing.
#[derive(Parser)]
#[command(author, version, about, long_about=None)]
pub struct ArgumentParser {
    #[arg(short, long, help = "the input file to operate on")]
    pub infile: PathBuf,

    #[arg(short, long, help = "the file to output the results to")]
    pub outfile: PathBuf,

    #[command(flatten)]
    pub verbose: Verbosity<InfoLevel>,
}

impl ArgumentParser {
    /// Set the log level based on the verbosity passed in.
    pub fn set_log_level(&self) {
        env_logger::Builder::new()
            .filter_level(self.verbose.log_level_filter())
            .init();
    }
}

/// A representation of the file formats that this library can process.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum FileFormat {
    /// Represents a LaTeX (.tex) file
    Tex,

    /// Represents a Markdown (.md) file
    Markdown,
}

impl FileFormat {
    /** Determine the file format from a given path.

    # Arguments

    * `p` - a reference to a path object

    # Return

    * `Ok(ext: FileFormat)` if the extension could be determined;
    * `Err(_)` otherwise

    # Examples:

    * With a valid extension:
    ```
    # use lilscript::FileFormat;
    # use std::path::PathBuf;
    let p = PathBuf::from(r"/home/user/Documents/f.tex");
    let file_format = FileFormat::from_path(&p).unwrap();
    assert_eq!(file_format, FileFormat::Tex);
    ```

    * With an invalid extension:
    ```
    # use lilscript::FileFormat;
    # use std::path::PathBuf;
    let p = PathBuf::from(r"/home/user/Documents/g.csv");
    let file_format = FileFormat::from_path(&p);
    assert!(file_format.is_err());
    ```
    */
    pub fn from_path(p: &PathBuf) -> Result<Self, String> {
        match p.extension() {
            Some(ext) => match ext.to_str() {
                Some("tex") => Ok(Self::Tex),
                Some("md") => Ok(Self::Markdown),
                _ => Err("Invalid file extension: should be .tex / .md".to_owned()),
            },
            None => Err("Invalid file extension: could not be determined".to_owned()),
        }
    }
}

// pub fn export_script(script: &Script, file_format: &FileFormat) -> String {
//     match file_format {
//         FileFormat::Tex => String::new(),
//         FileFormat::Markdown => script.to_markdown()
//     }
// }

pub fn run(args: ArgumentParser) -> Result<(), Box<dyn Error>> {
    let in_extension = FileFormat::from_path(&args.infile)?;
    let out_extension = FileFormat::from_path(&args.outfile)?;

    debug!("{:?} -> {:?}", in_extension, out_extension);

    if !(in_extension == FileFormat::Tex && out_extension == FileFormat::Markdown) {
        return Err("Only doing TeX ⟶ Markdown".into());
    }

    debug!("Reading from: {:?}", args.infile);
    let fcontents = fs::read_to_string(&args.infile)?;

    let script = match in_extension {
        FileFormat::Tex => {
            let tex = Tex::from(fcontents.as_str());
            Script::try_from(&tex)
        }
        _ => unreachable!(),
    }?;

    // // Get the exported file format
    // let script = crate::tex_handler::parse(&fcontents)?;

    // info!("Title: {}", script.title);
    // info!("Words: {}", script.wordcount());

    // Write the desired file
    // println!("{}", &script.to_string());
    fs::write(args.outfile, &script.to_markdown())?;

    Ok(())
}
