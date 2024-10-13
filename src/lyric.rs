use anyhow::Result;
use regex::Regex;
use std::{slice::Iter, time::Duration};

#[derive(serde::Deserialize, Debug, Default)]
#[allow(dead_code)]
struct LycResponse {
    version: i32,
    lyric: String,
}

#[derive(serde::Deserialize, Debug)]
#[allow(dead_code)]
struct NeteaseLyricsResponse {
    lrc: LycResponse,
    klyric: Option<LycResponse>,
    tlyric: Option<LycResponse>,
}

#[derive(Debug, Clone, Default)]
pub struct Lyrics {
    data: Vec<(Duration, String)>,
}

impl From<&String> for Lyrics {
    fn from(value: &String) -> Self {
        let re = Regex::new(r"\[(\d{2}):(\d{2})\.(\d{1,3})\]").unwrap();
        let mut lyrics: Vec<(Duration, String)> = Vec::new();
        for line in value.lines() {
            let mut last: usize = 0;
            let durations: Vec<_> = re
                .captures_iter(line)
                .filter_map(|x| {
                    let groups = (x.get(0), x.get(1), x.get(2), x.get(3));
                    match groups {
                        (Some(all), Some(minutes), Some(seconds), Some(millis)) => {
                            last = all.end();
                            let minutes: u64 = minutes.as_str().parse().unwrap_or_default();
                            let seconds: u64 = seconds.as_str().parse().unwrap_or_default();
                            let millis: u32 = millis.as_str().parse().unwrap_or_default();
                            Some(Duration::new(minutes * 60 + seconds, millis * 1_000_000))
                        }
                        _ => None,
                    }
                })
                .collect();
            if durations.is_empty() {
                continue;
            }
            let (_, lyric) = line.split_at(last);
            for duartion in durations {
                lyrics.push((duartion, lyric.into()));
            }
        }
        lyrics.sort_by_key(|&(time, _)| time);
        Lyrics { data: lyrics }
    }
}

impl Lyrics {
    /// # `from_netease_api` 从网易云api获取歌词
    /// 元祖内容依次为 `原歌词` `翻译歌词` `注音歌词`
    pub fn from_netease_api(id: &str) -> Result<(Option<Lyrics>, Option<Lyrics>, Option<Lyrics>)> {
        let url = format!("https://music.163.com/api/song/lyric?id={id}&lv=1&kv=1&tv=-1");
        dbg!(&url);
        let response: NeteaseLyricsResponse = reqwest::blocking::get(url)?.json()?;
        Ok((
            Some(Lyrics::from(&response.lrc.lyric)),
            Some(Lyrics::from(&response.tlyric.unwrap_or_default().lyric)),
            Some(Lyrics::from(&response.klyric.unwrap_or_default().lyric)),
        ))
    }

    #[allow(dead_code)]
    pub fn get_lyric(&self, time: Duration) -> Option<String> {
        self.iter()
            .filter(|x| x.0 < time)
            .last()
            .map(|x| x.1.clone())
    }

    pub fn get_no_space_lyric(&self, time: Duration) -> Option<String> {
        self.iter()
            .filter(|x| x.0 < time && !x.1.trim().is_empty())
            .last()
            .map(|x| x.1.clone())
    }

    pub fn iter(&self) -> Iter<(Duration, String)> {
        self.data.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}
