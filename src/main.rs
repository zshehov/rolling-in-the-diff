use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::PathBuf;

use bincode2::{deserialize, serialize};
use clap::Parser;
use env_logger::Env;
use log::info;

use rolling_in_the_diff::delta_generation::{generate_delta, Delta};
use rolling_in_the_diff::patch::patch;
use rolling_in_the_diff::rolling_checksum::rolling_adler32::RollingAdler32;
use rolling_in_the_diff::signature_generation::generate_signature;
use rolling_in_the_diff::strong_hash::md5::Md5Sum;
use rolling_in_the_diff::strong_hash::StrongHash;
use rolling_in_the_diff::Signature;

#[derive(Parser, Debug)]
#[clap(version, about)]
/// Simple CLI tool that tries to replicate rdiff's signature and delta commands
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Generates a signature of --old-file=<OLD_FILE> into --signature-file=<SIGNATURE_FILE> to be later used as a source for the "delta" command
    Signature {
        #[clap(long)]
        /// The source file of the signature
        old_file: PathBuf,
        #[clap(long)]
        /// The resulting signature file
        signature_file: PathBuf,
    },
    /// Generates the delta between a file described by --signature-file=<SIGNATURE_FILE> and a --new-file=<NEW_FILE> to --delta-file=<DELTA_FILE>
    Delta {
        #[clap(long)]
        /// The signature file describing the original content
        signature_file: PathBuf,
        #[clap(long)]
        /// The file with the (potentially) updated content
        new_file: PathBuf,
        #[clap(long)]
        /// The resulting delta file
        delta_file: PathBuf,
    },
    /// Applies --delta-file=<DELTA_FILE> on top of --old-file=<OLD_FILE> (not in place) and produces --updated_file<UPDATED_FILE>
    Patch {
        #[clap(long)]
        /// The delta file to apply
        delta_file: PathBuf,
        #[clap(long)]
        /// The file the delta is going to be applied on
        old_file: PathBuf,
        #[clap(long)]
        /// The file with the (potentially) updated content
        updated_file: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let cli: Cli = Cli::parse();

    return match cli.command {
        Commands::Signature {
            old_file,
            signature_file,
        } => {
            info!(
                "Generating signature of {} into {}",
                old_file.display(),
                signature_file.display()
            );

            let mut old_file = File::open(old_file)?;
            let mut old_file_content = Vec::<u8>::new();
            old_file.read_to_end(&mut old_file_content)?;

            let mut signature_file = File::create(signature_file)?;

            let signature = generate_signature::<RollingAdler32, Md5Sum>(&old_file_content);

            signature_file.write_all(serialize(&signature)?.as_slice())?;
            Ok(())
        }
        Commands::Delta {
            signature_file,
            new_file,
            delta_file,
        } => {
            info!(
                "Generating the delta between {} and {} into {}",
                signature_file.display(),
                new_file.display(),
                delta_file.display(),
            );

            let mut signature_file = File::open(signature_file)?;
            let mut signature_file_content = Vec::<u8>::new();
            signature_file.read_to_end(&mut signature_file_content)?;

            let signature: Signature<
                <RollingAdler32 as rolling_in_the_diff::rolling_checksum::RollingChecksum>::ChecksumType,
                <Md5Sum as StrongHash>::HashType
            > = deserialize(signature_file_content.as_slice())?;

            let mut new_file = File::open(new_file)?;
            let mut new_file_content = Vec::<u8>::new();
            new_file.read_to_end(&mut new_file_content)?;

            // TODO: signature_file and new_file processing should be done in separate threads

            let delta =
                generate_delta::<RollingAdler32, Md5Sum>(&signature, new_file_content.as_slice());

            let mut delta_file = File::create(delta_file)?;

            delta_file.write_all(serialize(&delta)?.as_slice())?;
            Ok(())
        }
        Commands::Patch {
            delta_file,
            old_file,
            updated_file,
        } => {
            info!(
                "Applying delta {} on top of {} into {}",
                delta_file.display(),
                old_file.display(),
                updated_file.display(),
            );

            let mut old_file = File::open(old_file)?;
            let mut old_file_content = Vec::<u8>::new();
            old_file.read_to_end(&mut old_file_content)?;

            let mut delta_file = File::open(delta_file)?;
            let mut delta_file_content = Vec::<u8>::new();
            delta_file.read_to_end(&mut delta_file_content)?;

            let delta: Delta<<Md5Sum as StrongHash>::HashType> =
                deserialize(delta_file_content.as_slice())?;
            let out_file = File::create(updated_file)?;

            patch::<Md5Sum, BufWriter<File>>(
                old_file_content.as_slice(),
                delta,
                &mut BufWriter::new(out_file),
            );
            Ok(())
        }
    };
}
