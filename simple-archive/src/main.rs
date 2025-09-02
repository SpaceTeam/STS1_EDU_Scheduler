use clap::Parser;
use std::{
    io::{Cursor, Read},
    path::PathBuf,
};

#[derive(clap::Parser, Debug)]
/// A small tool to extract simple-archives which are returned by the EDU scheduler
struct Args {
    /// Archive to extract
    file: PathBuf,

    #[arg(short, long)]
    /// Path into which the archive is extracted
    output: Option<PathBuf>,

    #[arg(long, action)]
    /// Do not decompress GZIP encoded data
    no_decompress: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let output_path = args.output.unwrap_or(PathBuf::from("."));
    let file = std::fs::File::open(args.file)?;
    let reader = simple_archive::Reader::new(file);

    for file in reader {
        match file {
            Ok(f) => {
                let data = if args.no_decompress {
                    f.data
                } else {
                    let mut decoder = flate2::read::GzDecoder::new(Cursor::new(f.data));
                    if decoder.header().is_some() {
                        let mut buffer = Vec::new();
                        decoder.read_to_end(&mut buffer)?;
                        buffer
                    } else {
                        decoder.into_inner().into_inner()
                    }
                };

                std::fs::write(output_path.join(f.path), data)?;
            }
            Err(e) => eprintln!("Failed to parse entry {e}"),
        }
    }

    Ok(())
}
