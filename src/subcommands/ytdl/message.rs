use nom::{branch::alt, bytes::complete::tag, sequence::tuple, IResult};
use phf::phf_map;

#[derive(Debug, Clone, PartialEq)]
pub struct DownloadProgress {
    percent: u8,
    total_size: u32,
    download_speed: u32,
    eta: u32, // TODO: format?
}

#[derive(Debug, Clone, PartialEq)]
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

static PARSER_PREFIXES: phf::Map<
    &str,
    for<'a> fn(&'a str) -> Result<Message, Box<dyn std::error::Error + 'a>>,
> = phf_map! {
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
fn parse_prefix<'a>(input: &'a str) -> IResult<&'a str, &'a str> {
    alt((
        tag(YOUTUBE_PREFIX),
        tag(YOUTUBE_TAB_PREFIX),
        tag(DOWNLOAD_PREFIX),
        tag(ERROR_PREFIX),
    ))(input)
}

fn parse_youtube<'a>(input: &'a str) -> Result<Message, Box<dyn std::error::Error + 'a>> {
    Ok(Message::PlaylistDownloadDone)
}

fn parse_youtube_tab<'a>(input: &'a str) -> Result<Message, Box<dyn std::error::Error + 'a>> {
    let (remainder, word) = alt((
        tag::<&str, &str, nom::error::Error<&str>>("Extracting URL:"),
        tag("Playlist:"),
    ))(input)?;
    Ok(Message::PlaylistDownloadDone)
}

fn parse_download<'a>(input: &'a str) -> Result<Message, Box<dyn std::error::Error + 'a>> {
    Ok(Message::PlaylistDownloadDone)
}
fn parse_error<'a>(input: &'a str) -> Result<Message, Box<dyn std::error::Error + 'a>> {
    Ok(Message::PlaylistDownloadDone)
}

fn parse<'a>(input: &'a str) -> Result<Message, Box<dyn std::error::Error + 'a>> {
    let (remainder, word) = parse_prefix(input)?;

    // TODO: replace match with iteration over PARSER_PREFIXES + fallthrough for '_'

    match word {
        YOUTUBE_PREFIX => parse_youtube(remainder),
        YOUTUBE_TAB_PREFIX => parse_youtube_tab(remainder),
        DOWNLOAD_PREFIX => parse_download(remainder),
        ERROR_PREFIX => Ok(Message::VideoDownloadError(remainder.to_owned())),
        _ => Ok(Message::PlaylistDownloadDone),
    }
}

#[cfg(test)]
mod tests {
    use nom::IResult;

    use super::parse_prefix;

    #[test]
    fn nom_works() -> anyhow::Result<()> {
        let video_url = "[youtube] Extracting URL: <video-url>";
        let video_download_path = "[download] Destination: <video-download-path>";
        let video_download_progress =
            "[download] <percent>% of <total-size> at <download-speed> ETA <eta>";
        let video_download_done =
            "[download] 100% of <total-size> in <total-time> at <download-speed>";
        let video_download_error = "ERROR: [youtube] <video-id>: <error-message>";
        let playlist_url = "[youtube:tab] Extracting URL: <playlist-url>";
        let playlist_name = "[download] Downloading playlist: <playlist-name>";
        let playlist_video_count = "[youtube:tab] Playlist <playlist-name>: Downloading <n> items of <playlist-video-count>";
        let playlist_video_index =
            "[download] Downloading item <playlist-video-index> of <playlist-video-count>";
        let playlist_download_done = "[download] Finished downloading playlist: <playlist-name>";

        let (remainder, word) = parse_prefix(video_url)?;

        Ok(())
    }
}
