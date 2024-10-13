use anyhow::Result;
use clap::Parser;
use mpris::PlayerFinder;
use regex::Regex;
use std::{thread::sleep, time::Duration};
mod lyric;
#[derive(Debug)]
#[allow(dead_code)]
enum MyError {
    CouldNotGetValue,
    ValueTypeError,
    CouldNodParseID(String),
    CouldNotGetLyric,
}

impl std::fmt::Display for MyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MyError::CouldNotGetValue => {
                write!(f, "could not get metadata value")
            }
            MyError::ValueTypeError => {
                write!(f, "value type error")
            }
            MyError::CouldNodParseID(s) => {
                write!(f, "could not parse id from {}", s)
            }
            MyError::CouldNotGetLyric => {
                write!(f, "could not get lyric")
            }
        }
    }
}

impl std::error::Error for MyError {}

#[derive(Parser, Debug)]
struct Args {
    #[arg(short = 'i', long = "disable_title",action = clap::ArgAction::SetFalse)]
    show_title: bool,
    #[arg(short = 'l', long = "disable_lyrics",action = clap::ArgAction::SetFalse)]
    show_lyric: bool,
    #[arg(short = 't', long = "disable_translate_lyrics",action = clap::ArgAction::SetFalse)]
    show_translate_lyric: bool,
    #[arg(short = 'p', long = "disable_position",action = clap::ArgAction::SetFalse)]
    show_position: bool,
    #[arg(short = 's', long = "disable_status",action = clap::ArgAction::SetFalse)]
    show_status: bool,
    #[arg(long,action = clap::ArgAction::SetTrue)]
    prev: bool,
    #[arg(long,action = clap::ArgAction::SetTrue)]
    next: bool,
    #[arg(long,action = clap::ArgAction::SetTrue)]
    toggle: bool,
}

#[derive(serde::Serialize, Debug, Default)]
struct Output {
    class: String,
    text: String,
    tooltip: String,
}

fn get_title(data: &mpris::Metadata) -> Result<String> {
    match data.get("xesam:title") {
        None => Err(MyError::CouldNotGetValue.into()),
        Some(v) => {
            if v.is_string() {
                Ok(v.clone().into_string().unwrap())
            } else {
                Err(MyError::ValueTypeError.into())
            }
        }
    }
}

fn get_length(data: &mpris::Metadata) -> Result<Duration> {
    match data.get("mpris:length") {
        None => Err(MyError::CouldNotGetValue.into()),
        Some(v) => {
            if v.is_i64() {
                Ok(Duration::from_micros(v.clone().into_i64().unwrap() as u64))
            } else {
                Err(MyError::ValueTypeError.into())
            }
        }
    }
}

fn get_id(data: &mpris::Metadata) -> Result<String> {
    match data.get("mpris:trackid") {
        None => Err(MyError::CouldNotGetValue.into()),
        Some(v) => match v.clone().into_string() {
            None => Err(MyError::ValueTypeError.into()),
            Some(v) => {
                let re = Regex::new(r"/org/mpd/Tracks/(?<id>\d+)").unwrap();
                let id = re
                    .captures(&v)
                    .and_then(|cap| cap.name("id").map(|x| x.as_str()));
                match id {
                    None => Err(MyError::CouldNodParseID(v).into()),
                    Some(id) => Ok(String::from(id)),
                }
            }
        },
    }
}

fn get_status_text(player: &mpris::Player) -> Result<char> {
    let status = player.get_playback_status()?;
    match status {
        mpris::PlaybackStatus::Paused => Ok('▶'),
        mpris::PlaybackStatus::Playing => Ok(''),
        mpris::PlaybackStatus::Stopped => Ok(''),
    }
}

fn format_duration(position: Duration, length: Duration) -> String {
    format!(
        "{:02}:{:02}/{:02}:{:02}",
        position.as_secs() / 60,
        position.as_secs() % 60,
        length.as_secs() / 60,
        length.as_secs() % 60
    )
}

fn output_default(default: &str) {
    let output = Output {
        class: "lyrics".into(),
        tooltip: default.into(),
        text: "Modolet".into(),
    };
    let output = serde_json::to_string(&output).unwrap();
    println!("{}", output);
}

fn find_player() -> std::result::Result<mpris::Player, mpris::FindingError> {
    PlayerFinder::new()?.find_by_name("musicfox")
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut id = String::default();
    let mut lyrics = lyric::Lyrics::default();
    let mut tyrics = lyric::Lyrics::default();
    if args.toggle {
        find_player()?.play_pause()?;
        return Ok(());
    }
    if args.next {
        find_player()?.next()?;
        return Ok(());
    }
    if args.prev {
        find_player()?.previous()?;
        return Ok(());
    }

    let mut exec = || -> Result<()> {
        let player = find_player()?;
        let metadata = player.get_metadata()?;
        let current_id = get_id(&metadata)?;
        if current_id != id || lyrics.is_empty() {
            id = current_id;
            let (i_lyrics, i_tyrics, _) = lyric::Lyrics::from_netease_api(&id)?;
            lyrics = i_lyrics.unwrap_or_default();
            tyrics = i_tyrics.unwrap_or_default();
        }
        let mut output_str = String::new();
        let position = player.get_position()?;
        if args.show_status {
            output_str.push(get_status_text(&player)?);
            output_str.push(' ');
        }
        if args.show_title {
            output_str.push_str(&get_title(&metadata)?);
        }
        if args.show_position {
            output_str.push_str(" (");
            output_str.push_str(&format_duration(position, get_length(&metadata)?));
            output_str.push(')');
        }
        if args.show_lyric {
            let lyric = lyrics.get_no_space_lyric(position).unwrap_or_default();
            if !lyric.is_empty() {
                output_str.push_str(" - ");
                output_str.push_str(&lyric);
            }
        }
        if args.show_translate_lyric {
            let trans = &tyrics.get_no_space_lyric(position).unwrap_or_default();
            if !trans.is_empty() {
                output_str.push_str(" [");
                output_str.push_str(trans);
                output_str.push(']');
            }
        }

        let output = Output {
            class: "lyrics".into(),
            text: output_str,
            tooltip: String::default(),
        };
        let output = serde_json::to_string(&output).unwrap();
        println!("{}", output);
        Ok(())
    };
    loop {
        if let Err(e) = exec() {
            output_default(&e.to_string());
        }
        sleep(Duration::from_millis(200));
    }
}
