use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    fs::{File, OpenOptions},
    io::{BufReader, Read, Write},
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};
use tokio::time::sleep;

pub async fn download_with_progress_async(
    url: &str,
    out_path: &str,
    label: &str,
    max_retries: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    for attempt in 1..=max_retries {
        println!(
            "{}",
            format!("Downloading {} (Attempt {}/{})", label, attempt, max_retries).yellow()
        );

        // Resume support
        let mut resume_from = 0;
        if Path::new(out_path).exists() {
            resume_from = std::fs::metadata(out_path).map(|m| m.len()).unwrap_or(0);
            if resume_from > 0 {
                println!("Resuming download from {} bytes", resume_from);
            }
        }

        let out_path_owned = out_path.to_string();
        let url_owned = url.to_string();
        let label_owned = label.to_string();

        let result = tokio::task::spawn_blocking(move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            // Start curl command with range if resuming
            let mut cmd = Command::new("curl");
            cmd.arg("--location")
                .arg("--fail")
                .arg("--silent")
                .arg("--show-error")
                .arg("-H")
                .arg("User-Agent: laracli/1.0")
                .arg(url_owned.clone());

            if resume_from > 0 {
                cmd.arg("-r").arg(format!("{}-", resume_from));
            }

            cmd.stdout(Stdio::piped());

            let mut child = cmd.spawn().map_err(|e| {
                format!("Failed to start curl: {}", e)
            })?;

            let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;

            let mut file = if resume_from > 0 {
                OpenOptions::new().append(true).open(&out_path_owned)?
            } else {
                File::create(&out_path_owned)?
            };

            // No content-length from curl, so use a spinner-style progress
            let pb = ProgressBar::new_spinner();
            pb.set_message(format!("Downloading {}", label_owned));
            pb.enable_steady_tick(Duration::from_millis(100));
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {bytes} - {msg}")
                    .unwrap(),
            );

            let mut reader = BufReader::new(stdout);
            let mut buffer = [0u8; 8192];
            let mut downloaded = resume_from;

            loop {
                let n = reader.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                file.write_all(&buffer[..n])?;
                downloaded += n as u64;
                pb.set_position(downloaded);
            }

            pb.finish_with_message(format!("✅ {} downloaded successfully", label_owned));
            Ok(())
        }).await;

        match result {
            Ok(Ok(())) => return Ok(()),
            Ok(Err(e)) => {
                println!("❌ Download failed: {}", e);
                if attempt == max_retries {
                    return Err(e);
                }
            }
            Err(e) => {
                println!("❌ Task join error: {}", e);
                if attempt == max_retries {
                    return Err(format!("Background task failed: {}", e).into());
                }
            }
        }

        println!("Retrying in 10 seconds...");
        sleep(Duration::from_secs(10)).await;
    }

    Err("Download failed after all retries".into())
}
