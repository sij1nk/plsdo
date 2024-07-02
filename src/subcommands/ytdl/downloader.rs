use std::{
    io::{BufRead, BufReader},
    process::{Child, Command as StdCommand, ExitStatus, Stdio},
};

use super::{DownloadFormat, ProcessId};

pub trait Downloader {
    fn download(
        self,
        url: impl AsRef<str>,
        format: &DownloadFormat,
    ) -> anyhow::Result<(
        ProcessId,
        impl Iterator<Item = std::io::Result<String>>,
        impl DownloadWaitHandle,
    )>;
}

pub trait DownloadWaitHandle {
    fn wait(self) -> anyhow::Result<ExitStatus>;
}

pub struct YtdlDownloadWaitHandle {
    child_process: Child,
}

impl YtdlDownloadWaitHandle {
    fn new(child_process: Child) -> Self {
        Self { child_process }
    }
}

impl DownloadWaitHandle for YtdlDownloadWaitHandle {
    fn wait(mut self) -> anyhow::Result<ExitStatus> {
        let res = self.child_process.wait()?;
        Ok(res)
    }
}

pub struct YtdlDownloader {}

impl YtdlDownloader {
    pub fn new() -> Self {
        Self {}
    }
}

fn get_download_format_specifier(format: &DownloadFormat) -> &'static [&'static str] {
    match format {
        DownloadFormat::UpTo1440p => {
            &["-f", "bestvideo[height<=1440]+bestaudio/best[height<=1440]"]
        }
        DownloadFormat::UpTo1080p => {
            &["-f", "bestvideo[height<=1080]+bestaudio/best[height<=1080]"]
        }
        DownloadFormat::UpTo720p => &["-f", "bestvideo[height<=720]+bestaudio/best[height<=720]"],
        DownloadFormat::UpTo480p => &["-f", "bestvideo[height<=480]+bestaudio/best[height<=480]"],
        DownloadFormat::WorstVideo => &["-S", "+size,+br,+res,+fps"],
        DownloadFormat::AudioOnly => &["-x", "--audio-format", "mp3"],
    }
}

impl Downloader for YtdlDownloader {
    fn download(
        self,
        url: impl AsRef<str>,
        format: &DownloadFormat,
    ) -> anyhow::Result<(
        ProcessId,
        impl Iterator<Item = std::io::Result<String>>,
        impl DownloadWaitHandle,
    )> {
        let format_specifier = get_download_format_specifier(format);
        let mut child = StdCommand::new("yt-dlp")
            .args(
                [
                    format_specifier,
                    &["--progress", "--newline", "-r", "16384", url.as_ref()],
                ]
                .concat(),
            )
            .stdout(Stdio::piped())
            .spawn()?;
        let stdout = child.stdout.take().ok_or_else(|| {
            anyhow::anyhow!("Failed to get the standard output of the child process")
        })?;
        let bufreader = BufReader::new(stdout);
        let lines = bufreader.lines();

        let pid = child.id();

        Ok((pid, lines, YtdlDownloadWaitHandle::new(child)))
    }
}
