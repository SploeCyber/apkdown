use std::path::PathBuf;
use std::process::exit;

use clap::Parser;

use crate::apkpure::ApkPure;
use crate::provider::{Provider, download_streamed};

pub mod apkpure;
pub mod provider;

#[derive(Debug, Clone, clap::Parser)]
pub struct Args {
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
    #[arg(long)]
    /// Only print the version, then exit. Useful if you just want to fetch the latest version available.
    pub only_get_version: bool,
}

impl Args {
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let apk_pure = ApkPure {};

        let (url, version) = self.get_url(apk_pure).await?;

        if self.only_get_version {
            println!("{version}");
            return Ok(());
        }

        let parent_dir = self.output_path.parent();

        if let Some(parent) = parent_dir {
            std::fs::create_dir_all(parent)?;
        }

        let path = self.output_path;

        if std::fs::exists(&path)? && self.skip_if_exists {
            if !self.silent {
                println!("skipping apk download as {} already exists", path.display());
            }
            return Ok(());
        }

        let callback = (!self.silent).then_some(callback_fn as fn(usize, usize));

        let data = download_streamed(&url, callback).await?;

        if !self.silent {
            println!("saving apk to: {}", path.display());
        }

        std::fs::write(path, data)?;

        Ok(())
    }

    async fn get_url(
        &self,
        apk_pure: ApkPure,
    ) -> Result<(String, String), Box<dyn std::error::Error>> {
        if let Some(ref version) = self.version {
            let url = apk_pure
                .get_version_url(&self.package_name, &version)
                .await?;

            Ok((url, version.to_string()))
        } else {
            let (latest_version, url) = apk_pure
                .get_latest_version(&self.package_name)
                .await?
                .ok_or(std::io::Error::other(format!(
                    "no latest version found for package: {0}",
                    &self.package_name
                )))?;

            let url = if let Some(url) = url {
                url
            } else {
                apk_pure
                    .get_version_url(&self.package_name, &latest_version)
                    .await?
            };

            if !self.silent {
                println!("latest version: {latest_version}");
            }
            Ok((url, latest_version))
        }
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
