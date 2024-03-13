use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_until, take_while},
    character::{
        complete::{char, digit0, digit1, multispace0},
        is_digit,
    },
    combinator::{fail, map_res},
    multi::separated_list0,
    number::complete::double,
    sequence::preceded,
    IResult,
};
use phf::phf_map;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadProgress {
    percent: u32,
    total_size: u32,     // KiB, rounded
    download_speed: u32, // KiB/s, rounded
    eta: u32,            // seconds
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

// TODO: if parsing fails due to unexpected input format, we'd like to know:
// - what the input was
// - what the nom error was (where it got stuck)
pub fn parse(input: &str) -> anyhow::Result<Message> {
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

// "[youtube] Extracting URL: <video-url>";
//           ^
fn parse_youtube(input: &str) -> IResult<&str, Message> {
    let (rem, _) = tag(" Extracting URL: ")(input)?;
    Ok(("", Message::VideoUrl(rem.into())))
}

// "[youtube:tab] Playlist <playlist-name>: Downloading 99 items of 99";
// "[youtube:tab] Extracting URL: <playlist-url>";
//               ^
fn parse_youtube_tab(input: &str) -> IResult<&str, Message> {
    let extracting_prefix = " Extracting URL: ";
    let playlist_prefix = " Playlist ";

    let (rem, word) = alt((tag(extracting_prefix), tag(playlist_prefix)))(input)?;

    let mut u32_parser = map_res(digit1, |s: &str| s.parse::<u32>());

    match word {
        w if w == playlist_prefix => {
            let (rem, _) = take_until(":")(rem)?;
            let (rem, _) = tag(": Downloading ")(rem)?;
            let (rem, _) = digit0(rem)?;
            let (rem, _) = tag(" items of ")(rem)?;
            let (_, total_count) = u32_parser(rem)?;

            Ok(("", Message::PlaylistVideoCount(total_count)))
        }
        w if w == extracting_prefix => Ok(("", Message::PlaylistUrl(rem.into()))),
        _ => fail(rem),
    }
}

// "[download] Destination: <video-download-path>";
// "[download] 10% of 543.71KiB at 16.00KiB/s ETA 00:30";
// "[download] 100% of <total-size> in <total-time> at <download-speed>";
// "[download] Downloading playlist: <playlist-name>";
// "[download] Downloading item 11 of 99";
// "[download] Finished downloading playlist: <playlist-name>";
//            ^
fn parse_download(input: &str) -> IResult<&str, Message> {
    let video_download_path_prefix = " Destination: ";
    let playlist_name_prefix = " Downloading playlist: ";
    let playlist_video_index_prefix = " Downloading item ";
    let playlist_download_done_prefix = " Finished downloading playlist: ";

    let mut u32_parser = map_res(digit1, |s: &str| s.parse::<u32>());

    let (rem, word) = alt((
        tag(video_download_path_prefix),
        tag(playlist_name_prefix),
        tag(playlist_video_index_prefix),
        tag(playlist_download_done_prefix),
        preceded(multispace0, take_while(is_char_digit)),
    ))(input)?;

    match word {
        w if w == video_download_path_prefix => Ok(("", Message::VideoDownloadPath(rem.into()))),
        w if w == playlist_name_prefix => Ok(("", Message::PlaylistName(rem.into()))),
        w if w == playlist_video_index_prefix => {
            let (_, video_index) = u32_parser(rem)?;
            Ok(("", Message::PlaylistVideoIndex(video_index)))
        }
        w if w == playlist_download_done_prefix => Ok(("", Message::PlaylistDownloadDone)),
        _ => {
            let progress_percent = double(word)?.1.round() as u32;
            if progress_percent == 100 {
                return Ok(("", Message::VideoDownloadDone));
            }
            // "% of 543.71KiB at 16.00KiB/s ETA 00:30";
            let (rem, _) = tag("% of")(rem)?;
            let (rem, _) = multispace0(rem)?;
            let (rem, total_size_double) = double(rem)?;
            let (rem, total_size_m) = parse_size_measurement(rem)?;
            let (rem, _) = tag(" at")(rem)?;
            let (rem, _) = multispace0(rem)?;
            let (rem, download_speed_double) = double(rem)?; // TODO: sometimes we get "Unknown B/s here"
            let (rem, download_speed_m) = parse_size_measurement(rem)?;
            let (rem, _) = tag("/s ETA ")(rem)?;
            let (_, eta) = parse_eta(rem)?; // TODO: sometimes we get 'Unknown' here

            let total_size = get_size_in_kib(total_size_double, total_size_m);
            let download_speed = get_size_in_kib(download_speed_double, download_speed_m);
            let download_progress = DownloadProgress {
                percent: progress_percent,
                total_size,
                download_speed,
                eta,
            };
            Ok(("", Message::VideoDownloadProgress(download_progress)))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SizeMeasurement {
    B,
    KiB,
    MiB,
    GiB,
}

fn get_size_in_kib(size: f64, m: SizeMeasurement) -> u32 {
    let multiplier = match m {
        SizeMeasurement::B => return 0u32, // round down
        SizeMeasurement::KiB => 1,
        SizeMeasurement::MiB => 1024,
        SizeMeasurement::GiB => 1024 * 1024,
    } as f64;

    (size * multiplier).round() as u32
}

fn parse_size_measurement(input: &str) -> IResult<&str, SizeMeasurement> {
    let (rem, word) = alt((tag("B"), tag("KiB"), tag("MiB"), tag("GiB")))(input)?;

    let measurement = match word {
        "B" => Some(SizeMeasurement::B),
        "KiB" => Some(SizeMeasurement::KiB),
        "MiB" => Some(SizeMeasurement::MiB),
        "GiB" => Some(SizeMeasurement::GiB),
        _ => None,
    }
    .expect("size to be one of B, KiB, MiB, GiB");

    Ok((rem, measurement))
}

// [hh:]mm:ss
fn parse_eta(input: &str) -> IResult<&str, u32> {
    let u32_parser = map_res(digit1, |s: &str| s.parse::<u32>());
    let (_, list) = separated_list0(tag(":"), u32_parser)(input)?;
    let seconds = list
        .iter()
        .rev()
        .enumerate()
        .fold(0, |acc, (i, curr)| acc + (curr * 60u32.pow(i as u32)));
    Ok(("", seconds))
}

// "ERROR: [youtube] <video-id>: <error-message>" -> <error-message>
//        ^
fn parse_error(input: &str) -> IResult<&str, Message> {
    let (rem, _) = take_until(":")(input)?;
    let (rem, _) = take(2usize)(rem)?;
    Ok(("", Message::VideoDownloadError(rem.into())))
}

fn is_char_digit(c: char) -> bool {
    c.is_ascii() && (is_digit(c as u8) || c == '.')
}

#[cfg(test)]
mod tests {
    use crate::test_message_parsing;

    use super::*;

    const VIDEO_URL: &str = "[youtube] Extracting URL: <video-url>";
    const VIDEO_DOWNLOAD_PATH: &str = "[download] Destination: <video-download-path>";
    const VIDEO_DOWNLOAD_PROGRESS: &str =
        "[download]   0.2% of    9.84MiB at    4.40KiB/s ETA 41:56";
    const VIDEO_DOWNLOAD_DONE: &str = "[download] 100% of   14.25MiB in 00:00:01 at 9.29MiB/s";
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
                percent: 0,
                total_size: 10076,
                download_speed: 4,
                eta: 2516,
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

    #[test]
    fn parse_eta_works() -> anyhow::Result<()> {
        assert_eq!(parse_eta("00:00").unwrap().1, 0);
        assert_eq!(parse_eta("01:00").unwrap().1, 60);
        assert_eq!(parse_eta("01:59").unwrap().1, 119);
        assert_eq!(parse_eta("02:00").unwrap().1, 120);
        assert_eq!(parse_eta("01:00:00").unwrap().1, 3600);

        // we don't care if mm or ss >= 60, even though it's technically incorrect
        assert_eq!(parse_eta("00:69").unwrap().1, 69);
        assert_eq!(parse_eta("01:09").unwrap().1, 69);
        Ok(())
    }
}
