use std::path::PathBuf;
use std::process::exit;

use clap::Parser;

use crate::apkpure::ApkPure;
use crate::provider::{Provider, download_streamed};

pub mod apkpure;
pub mod provider;

#[derive(Debug, Clone, clap::Parser)]
pub struct DownloadArgs {
    /// The name of the package to download
    #[arg(long, short)]
    pub package_name: String,
    /// The version to download. Optional. If unspecified, the latest version will be downloaded
    #[arg(long, short)]
    pub version: Option<String>,
    /// Do not print any output to the console
    #[arg(long, short)]
    pub silent: bool,
    // Location to write the apk to. Creates any directories necessary.
    #[arg(long, short)]
    pub output_path: PathBuf,
    #[arg(long)]
    /// Skip downloading if the APK already exists
    pub skip_if_exists: bool,
}
impl DownloadArgs {
    async fn download(&self, apk_pure: ApkPure) -> Result<(), Box<dyn std::error::Error>> {
        let (url, version) = get_url(
            &self.package_name,
            self.version.as_ref().map(|v| v.as_str()),
            apk_pure,
        )
        .await?;

        if !self.silent {
            println!("downloading version: {version}")
        }

        let parent_dir = self.output_path.parent();

        if let Some(parent) = parent_dir {
            std::fs::create_dir_all(parent)?;
        }

        if std::fs::exists(&self.output_path)? && self.skip_if_exists {
            if !self.silent {
                println!(
                    "skipping apk download as {} already exists",
                    self.output_path.display()
                );
            }
            return Ok(());
        }

        let callback = (!self.silent).then_some(callback_fn as fn(usize, usize));

        let data = download_streamed(&url, callback).await?;

        if !self.silent {
            println!("saving apk to: {}", self.output_path.display());
        }

        std::fs::write(&self.output_path, data)?;

        Ok(())
    }
}

#[derive(Debug, Clone, clap::Parser)]
pub struct GetLatestArgs {
    /// The name of the package to download
    #[arg(long, short)]
    pub package_name: String,
}
impl GetLatestArgs {
    async fn get_latest(&self, apk_pure: ApkPure) -> Result<(), Box<dyn std::error::Error>> {
        let latest = apk_pure
            .get_latest_version(&self.package_name)
            .await?
            .ok_or(std::io::Error::other("no latest version found"))?
            .0;

        println!("{latest}");

        Ok(())
    }
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Command {
    Download(DownloadArgs),
    GetLatest(GetLatestArgs),
}

#[derive(Debug, Clone, clap::Parser)]
pub struct Args {
    #[command(subcommand)]
    pub subcommand: Command,
}

impl Args {
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let apk_pure = ApkPure {};

        match self.subcommand {
            Command::Download(download_args) => download_args.download(apk_pure).await,
            Command::GetLatest(get_latest_args) => get_latest_args.get_latest(apk_pure).await,
        }
    }
}

async fn get_url(
    package_name: &str,
    version: Option<&str>,
    apk_pure: ApkPure,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    if let Some(ref version) = version {
        let url = apk_pure.get_version_url(package_name, version).await?;

        Ok((url, version.to_string()))
    } else {
        let (latest_version, url) =
            apk_pure
                .get_latest_version(package_name)
                .await?
                .ok_or(std::io::Error::other(format!(
                    "no latest version found for package: {0}",
                    package_name
                )))?;

        let url = if let Some(url) = url {
            url
        } else {
            apk_pure
                .get_version_url(package_name, &latest_version)
                .await?
        };

        Ok((url, latest_version))
    }
}

fn callback_fn(current: usize, total: usize) {
    println!(
        "{current}/{total} {}%",
        current as f32 / total as f32 * 100.0
    );
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if let Err(e) = args.run().await {
        eprintln!("{e}");
        exit(1);
    }
}
