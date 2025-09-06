use std::path::PathBuf;
use std::process::exit;

use clap::Parser;

use crate::apkpure::ApkPure;
use crate::provider::{Provider, download_streamed};

pub mod apkpure;
pub mod provider;

#[derive(Debug, Clone, clap::Parser)]
pub struct Args {
    #[arg(long, short)]
    pub package_name: String,
    #[arg(long, short)]
    pub version: Option<String>,
    #[arg(long, short)]
    pub silent: bool,
    #[arg(long, short)]
    pub output_dir: PathBuf,
    #[arg(long)]
    pub skip_if_exists: bool,
    #[arg(long)]
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

        let path = self.output_dir.join(self.package_name);

        std::fs::create_dir_all(&path)?;

        let path = path.join(format!("{}.apk", version));

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
