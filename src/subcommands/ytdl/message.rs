use nom::{
    branch::alt, bytes::complete::tag, character::complete::multispace0, sequence::tuple, IResult,
};
use phf::phf_map;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadProgress {
    percent: u8,
    total_size: u32,     // KiB, rounded
    download_speed: u32, // KiB/s, rounded
    eta: u32,            // TODO: format? // seconds
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    VideoUrl(String),                        // [youtube] Extracting URL: <video-url>
    VideoDownloadPath(String),               // [download] Destination: <video-download-path>
    VideoDownloadProgress(DownloadProgress), // [download] <percent>% of <total-size> at <download-speed> ETA <eta>
    VideoDownloadDone, // [download] 100% of <total-size> in <total-time> at <download-speed>
    VideoDownloadError(String), // ERROR: [youtube] <video-id>: <error-message>
    PlaylistUrl(String), // [youtube:tab] Extracting URL: <playlist-url>
    PlaylistName(String), // [download] Downloading playlist: <playlist-name>
    PlaylistVideoCount(u32), // [youtube:tab] Playlist <playlist-name>: Downloading <n> items of <playlist-video-count>
    PlaylistVideoIndex(u32), // [download] Downloading item <playlist-video-index> of <playlist-video-count>
    PlaylistDownloadDone,    // [download] Finished downloading playlist: <playlist-name>
}

impl TryFrom<&str> for Message {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        todo!()
    }
}

static PARSER_PREFIXES: phf::Map<&str, for<'a> fn(&'a str) -> IResult<&'a str, Message>> = phf_map! {
    "[download]" => parse_download,
    "[youtube]" => parse_youtube,
    "[youtube:tab]" => parse_youtube_tab,
    "ERROR:" => parse_error
};

// TODO: maybe remove these, or use them in the keys of PARSER_PREFIXES, if keeping them is necessary
const DOWNLOAD_PREFIX: &str = "[download]";
const YOUTUBE_PREFIX: &str = "[youtube]";
const YOUTUBE_TAB_PREFIX: &str = "[youtube:tab]";
const ERROR_PREFIX: &str = "ERROR:";

// TODO: Would be nice if we could get rid of this somehow
// macro would possibly work and could be reusable in other parsing situations too
fn parse_prefix(input: &str) -> IResult<&str, &str> {
    alt((
        tag(YOUTUBE_PREFIX),
        tag(YOUTUBE_TAB_PREFIX),
        tag(DOWNLOAD_PREFIX),
        tag(ERROR_PREFIX),
    ))(input)
}

fn parse_youtube(input: &str) -> IResult<&str, Message> {
    Ok(("", Message::PlaylistDownloadDone))
}

fn parse_youtube_tab(input: &str) -> IResult<&str, Message> {
    let (remainder, word) = alt((
        tag::<&str, &str, nom::error::Error<&str>>("Extracting URL:"),
        tag("Playlist:"),
    ))(input)?;
    Ok(("", Message::PlaylistDownloadDone))
}

fn parse_download(input: &str) -> IResult<&str, Message> {
    Ok(("", Message::PlaylistDownloadDone))
}

// "ERROR: [youtube] <video-id>: <error-message>" -> <error-message>
fn parse_error(input: &str) -> IResult<&str, Message> {
    let (remainder, _) = multispace0(input)?;
    let (remainder, _) = tag("[youtube]")(input)?;
    Ok(("", Message::PlaylistDownloadDone))
}

fn parse(input: &str) -> anyhow::Result<Message> {
    let (rem, prefix) = parse_prefix(input).map_err(|e| e.to_owned())?;

    if let Some(rem_parser) = PARSER_PREFIXES
        .entries()
        .find(|(k, _)| k == &&prefix)
        .map(|(_, v)| v)
    {
        rem_parser(rem)
            .map(|rem| rem.1)
            .map_err(|e| e.to_owned().into())
    } else {
        Err(anyhow::anyhow!("Unexpected parser prefix: '{}'", prefix))
    }
}

#[cfg(test)]
mod tests {
    use crate::test_message_parsing;

    use super::*;

    const VIDEO_URL: &str = "[youtube] Extracting URL: <video-url>";
    const VIDEO_DOWNLOAD_PATH: &str = "[download] Destination: <video-download-path>";
    const VIDEO_DOWNLOAD_PROGRESS: &str = "[download] 10% of 543.71KiB at 16.00KiB/s ETA 00:30";
    const VIDEO_DOWNLOAD_DONE: &str =
        "[download] 100% of <total-size> in <total-time> at <download-speed>";
    const VIDEO_DOWNLOAD_ERROR: &str = "ERROR: [youtube] <video-id>: <error-message>";
    const PLAYLIST_URL: &str = "[youtube:tab] Extracting URL: <playlist-url>";
    const PLAYLIST_NAME: &str = "[download] Downloading playlist: <playlist-name>";
    const PLAYLIST_VIDEO_COUNT: &str =
        "[youtube:tab] Playlist <playlist-name>: Downloading 99 items of 99";
    const PLAYLIST_VIDEO_INDEX: &str = "[download] Downloading item 11 of 99";
    const PLAYLIST_DOWNLOAD_DONE: &str =
        "[download] Finished downloading playlist: <playlist-name>";

    #[test]
    fn parse_prefix_works() -> anyhow::Result<()> {
        let s = format!("{} yada yada", YOUTUBE_PREFIX);
        assert!(parse_prefix(&s).is_ok());

        let s2 = "[Unrecognized prefix]";
        assert!(parse_prefix(s2).is_err());

        Ok(())
    }

    test_message_parsing!(
        video_url: (VIDEO_URL, Message::VideoUrl("<video-url>".into())),
        video_download_path: (
            VIDEO_DOWNLOAD_PATH,
            Message::VideoDownloadPath("<video-download-path>".into()),
        ),
        video_download_progress: (
            VIDEO_DOWNLOAD_PROGRESS,
            Message::VideoDownloadProgress(DownloadProgress {
                percent: 10,
                total_size: 544,
                download_speed: 16,
                eta: 30,
            }),
        ),
        video_download_done: (VIDEO_DOWNLOAD_DONE, Message::VideoDownloadDone),
        video_download_error: (
            VIDEO_DOWNLOAD_ERROR,
            Message::VideoDownloadError("<error-message>".into()),
        ),
        playlist_url: (PLAYLIST_URL, Message::PlaylistUrl("<playlist-url>".into())),
        playlist_name: (
            PLAYLIST_NAME,
            Message::PlaylistName("<playlist-name>".into()),
        ),
        playlist_video_count: (PLAYLIST_VIDEO_COUNT, Message::PlaylistVideoCount(99)),
        playlist_video_index: (PLAYLIST_VIDEO_INDEX, Message::PlaylistVideoIndex(11)),
        playlist_download_done: (PLAYLIST_DOWNLOAD_DONE, Message::PlaylistDownloadDone),
    );
}
