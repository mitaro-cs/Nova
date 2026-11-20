//! Р’СЃС‚СЂРѕРµРЅРЅС‹Рµ РїР»РµРµСЂС‹ РєР°С‚Р°Р»РѕРіРѕРІ С„РёР»СЊРјРѕРІ/СЃРµСЂРёР°Р»РѕРІ (LordFilm-РєР»РѕРЅС‹ Рё СЂРѕРґСЃС‚РІРµРЅРЅС‹Рµ):
//! СЃС‚СЂР°РЅРёС†Р° С„РёР»СЊРјР°/СЃРµСЂРёР°Р»Р° в†’ `<iframe>` РїР»РµРµСЂР° в†’ РІ РµРіРѕ HTML Р»РµР¶РёС‚ РѕРїРёСЃР°РЅРёРµ РІРёРґРµРѕ:
//! Р»РёР±Рѕ `seasons:[{вЂ¦episodes:[вЂ¦]}]` (СЃРµСЂРёР°Р»С‹), Р»РёР±Рѕ `source:{вЂ¦}` (С„РёР»СЊРј).
//! РЈ СЃРµСЂРёРё: `hls` (master.m3u8), `audio.names`/`order` (РѕР·РІСѓС‡РєРё), `cc` (СЃСѓР±С‚РёС‚СЂС‹ VTT),
//! `duration`. РЈ С„РёР»СЊРјР° - С‚Рѕ Р¶Рµ РІ `source`. DASH РЅРµ Р±РµСЂС‘Рј: hls.js РµРіРѕ РЅРµ РёРіСЂР°РµС‚.

use reqwest::Url;
use serde_json::Value;
use std::time::Duration;

/// РћРґРЅР° СЃРµСЂРёСЏ РёР»Рё С„РёР»СЊРј РёР· РїР»РµРµСЂР°.
#[derive(Debug, Clone)]
pub struct Media {
    pub hls: String,
    /// РРјРµРЅР° РѕР·РІСѓС‡РµРє РІ РїРѕСЂСЏРґРєРµ РґРѕСЂРѕР¶РµРє (`rus0`, `rus1`вЂ¦ РІ РїР»РµР№Р»РёСЃС‚Рµ).
    pub audio: Vec<String>,
    /// РЎСѓР±С‚РёС‚СЂС‹: (РёРјСЏ, Р°РґСЂРµСЃ VTT).
    pub subs: Vec<(String, String)>,
    pub duration: Option<i64>,
}

/// `#nova-season=S&episode=E` РІ СЃСЃС‹Р»РєРµ - РєР°РєСѓСЋ СЃРµСЂРёСЋ РёРіСЂР°С‚СЊ (РєР°Рє Сѓ LordFilm).
pub fn episode_target(u: &Url) -> Option<(i64, i64)> {
    let fragment = u.fragment()?;
    let (season, episode) = fragment.strip_prefix("nova-season=")?.split_once("&episode=")?;
    let (season, episode) = (season.parse::<i64>().ok()?, episode.parse::<i64>().ok()?);
    ((0..=1000).contains(&season) && (1..=10000).contains(&episode)).then_some((season, episode))
}

fn decode(s: &str) -> String {
    s.replace("&quot;", "\"").replace("&#039;", "'").replace("&nbsp;", " ").replace("&amp;", "&")
}

fn valid_hls(s: &str) -> bool {
    Url::parse(s).is_ok_and(|u| {
        crate::outbound::valid_url(&u) && (u.path().ends_with(".m3u8") || u.path().ends_with(".mpd"))
    })
}

fn number(v: &Value) -> Option<i64> {
    v.as_i64().or_else(|| v.as_str()?.parse::<i64>().ok())
}

/// `audio:{"names":[вЂ¦],"order":[вЂ¦]}` в†’ РёРјРµРЅР° РѕР·РІСѓС‡РµРє РїРѕ РёРЅРґРµРєСЃСѓ РґРѕСЂРѕР¶РєРё.
fn audio_names(v: Option<&Value>) -> Vec<String> {
    let Some(a) = v else { return vec![] };
    let names: Vec<String> = a["names"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|n| n.as_str().map(str::to_string))
        .collect();
    if names.is_empty() {
        return vec![];
    }
    // `order[i]` - РЅРѕРјРµСЂ РёРјРµРЅРё РґР»СЏ РґРѕСЂРѕР¶РєРё i; Р±РµР· order РёРјРµРЅР° РёРґСѓС‚ РїРѕ РїРѕСЂСЏРґРєСѓ.
    let order: Vec<usize> = a["order"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|n| n.as_u64().map(|x| x as usize))
        .collect();
    if order.is_empty() {
        return names;
    }
    (0..order.len()).map(|i| names.get(order[i]).cloned().unwrap_or_default()).filter(|n| !n.is_empty()).collect()
}

fn subs(v: Option<&Value>) -> Vec<(String, String)> {
    v.and_then(|c| c.as_array())
        .into_iter()
        .flatten()
        .filter_map(|s| {
            let url = s["url"].as_str().filter(|u| Url::parse(u).is_ok_and(|u| crate::outbound::valid_url(&u)))?;
            let name = s["name"].as_str().unwrap_or("РЎСѓР±С‚РёС‚СЂС‹").to_string();
            Some((name, url.to_string()))
        })
        .take(12)
        .collect()
}

fn media_of(v: &Value) -> Option<Media> {
    let hls = v["hls"].as_str().or_else(|| v["dash"].as_str()).filter(|u| valid_hls(u))?;
    Some(Media {
        hls: hls.to_string(),
        audio: audio_names(v.get("audio")),
        subs: subs(v.get("cc")),
        duration: v["duration"].as_i64().filter(|d| *d > 0),
    })
}

/// РњРµРґРёР° РёР· HTML РІСЃС‚СЂРѕРµРЅРЅРѕРіРѕ РїР»РµРµСЂР°: СЃРµСЂРёСЏ РїРѕ С„СЂР°РіРјРµРЅС‚Сѓ `#nova-season/episode` РёР»Рё С„РёР»СЊРј
/// РёР· `source:{вЂ¦}`. None - СЌС‚РѕС‚ HTML РЅРµ РѕРїРёСЃР°РЅРёРµ РІРёРґРµРѕ (РѕР±С‹С‡РЅР°СЏ СЃС‚СЂР°РЅРёС†Р°, С‡СѓР¶РѕР№ РїР»РµРµСЂ).
pub fn media_for(html: &str, target: Option<(i64, i64)>) -> Option<Media> {
    if let Some((s, e)) = target {
        return episodes(html).into_iter().find(|(ps, pe, _)| (*ps, *pe) == (s, e)).map(|(_, _, m)| m);
    }
    single(html)
}

/// Р’ HTML РµСЃС‚СЊ РѕРїРёСЃР°РЅРёРµ СЃРµСЂРёР°Р»Р° `seasons:[вЂ¦]` (РґР°Р¶Рµ РµСЃР»Рё РЅСѓР¶РЅРѕР№ СЃРµСЂРёРё РІ РЅС‘Рј РЅРµС‚).
pub fn has_series(html: &str) -> bool {
    regex::Regex::new(r"\bseasons\s*:\s*\[").is_ok_and(|re| re.is_match(html))
}

/// Р’СЃРµ СЃРµСЂРёРё РёР· `seasons:[вЂ¦]` (С‚Рѕ, С‡С‚Рѕ СѓР¶Рµ СѓРјРµРµС‚ lordfilm.rs, РЅРѕ СЃ РѕР·РІСѓС‡РєР°РјРё Рё СЃСѓР±С‚РёС‚СЂР°РјРё).
fn episodes(html: &str) -> Vec<(i64, i64, Media)> {
    let Ok(re) = regex::Regex::new(r"\bseasons\s*:\s*(\[)") else { return vec![] };
    let mut out = Vec::new();
    for capture in re.captures_iter(html) {
        let Some(start) = capture.get(1).map(|m| m.start()) else { continue };
        let Some(Ok(seasons)) = serde_json::Deserializer::from_str(&html[start..]).into_iter::<Value>().next() else {
            continue;
        };
        for s in seasons.as_array().into_iter().flatten() {
            let Some(season) = number(&s["season"]) else { continue };
            if !(0..=1000).contains(&season) || s["blocked"] == true {
                continue;
            }
            for e in s["episodes"].as_array().into_iter().flatten() {
                let Some(episode) = number(&e["episode"]) else { continue };
                if !(1..=10000).contains(&episode) || e["blocked"] == true {
                    continue;
                }
                if let Some(m) = media_of(e) {
                    out.push((season, episode, m));
                }
            }
        }
    }
    out
}

/// РљРѕРЅРµС† СЃР±Р°Р»Р°РЅСЃРёСЂРѕРІР°РЅРЅРѕРіРѕ `{вЂ¦}`/`[вЂ¦]` РЅР°С‡РёРЅР°СЏ СЃ РѕС‚РєСЂС‹РІР°СЋС‰РµР№ СЃРєРѕР±РєРё.
fn balanced(html: &str, open: usize) -> Option<usize> {
    let bytes = html.as_bytes();
    let (open_ch, close_ch) = match bytes.get(open)? {
        b'{' => (b'{', b'}'),
        b'[' => (b'[', b']'),
        _ => return None,
    };
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    for (i, &b) in bytes.iter().enumerate().skip(open) {
        if in_str {
            if esc {
                esc = false;
            } else if b == b'\\' {
                esc = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b if b == open_ch => depth += 1,
            b if b == close_ch => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
    }
    None
}

/// `key: "Р·РЅР°С‡РµРЅРёРµ"` РІ JS-РѕР±СЉРµРєС‚Рµ (РєР»СЋС‡ Р±РµР· РєР°РІС‹С‡РµРє): РІРµСЂРЅСѓС‚СЊ РґРµРєРѕРґРёСЂРѕРІР°РЅРЅСѓСЋ СЃС‚СЂРѕРєСѓ.
fn js_str(html: &str, key: &str) -> Option<String> {
    let pat = format!("{key}:");
    let at = html.find(&pat)? + pat.len();
    let rest = html[at..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let b = rest.as_bytes();
    let mut end = 0;
    while end < b.len() {
        match b[end] {
            b'\\' => end += 2,
            b'"' => break,
            _ => end += 1,
        }
    }
    serde_json::from_str::<String>(&format!("\"{}\"", rest.get(..end)?)).ok()
}

/// `key: {вЂ¦}` РёР»Рё `key: [вЂ¦]` - СЃС‹СЂРѕР№ JSON-С„СЂР°РіРјРµРЅС‚ (РІРЅСѓС‚СЂРё РєР°РІС‹С‡РєРё Сѓ РєР»СЋС‡РµР№ РµСЃС‚СЊ).
fn js_json(html: &str, key: &str) -> Option<String> {
    let pat = format!("{key}:");
    let at = html.find(&pat)? + pat.len();
    let rest = html[at..].trim_start();
    let open = rest.find(['{', '['])?;
    let end = balanced(rest, open)?;
    Some(rest[..end].to_string())
}

/// Р¤РёР»СЊРј: `makePlayer({вЂ¦, source:{ dash, dasha, hls, audio:{вЂ¦}, cc:[вЂ¦] }})`.
fn single(html: &str) -> Option<Media> {
    // `source:` РјРѕР¶РµС‚ РІСЃС‚СЂРµС‡Р°С‚СЊСЃСЏ Рё РІ РґСЂСѓРіРёС… РєРѕРЅС‚РµРєСЃС‚Р°С… - Р±РµСЂС‘Рј РїРµСЂРІС‹Р№ РѕР±СЉРµРєС‚ СЃ hls.
    let mut from = 0;
    while let Some(at) = html[from..].find("source:") {
        // В«hls_source:В», В«video_source:В» - РЅРµ РЅР°С€ РѕР±СЉРµРєС‚.
        let before = html[..from + at].chars().last();
        if before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_') {
            from += at + "source:".len();
            continue;
        }
        let start = from + at + "source:".len();
        let rest = html[start..].trim_start();
        let Some(open) = rest.find('{') else { break };
        let Some(end) = balanced(rest, open) else { break };
        let obj = &rest[..end];
        if let Some(hls) = js_str(obj, "hls").filter(|u| valid_hls(u)) {
            let audio = js_json(obj, "audio").and_then(|j| serde_json::from_str::<Value>(&j).ok()).map(|v| audio_names(Some(&v))).unwrap_or_default();
            let subs = js_json(obj, "cc")
                .and_then(|j| serde_json::from_str::<Value>(&j).ok())
                .map(|v| subs(Some(&v)))
                .unwrap_or_default();
            let duration = js_str(obj, "duration").and_then(|d| d.parse::<i64>().ok()).filter(|d| *d > 0);
            return Some(Media { hls, audio, subs, duration });
        }
        from = start + open;
    }
    None
}

/// РђРґСЂРµСЃР°, РЅР° РєРѕС‚РѕСЂС‹Рµ РјРѕР¶РЅРѕ С…РѕРґРёС‚СЊ Р·Р° РїРѕС‚РѕРєРѕРј: РґРѕРјРµРЅ СЃС‚СЂР°РЅРёС†С‹ + CDN РёР· СЃСЃС‹Р»РѕРє РјРµРґРёР°.
fn extra_hosts(page: &Url, media: &Media, frames: &[Url]) -> Vec<String> {
    let mut out: Vec<String> = frames.iter().filter_map(crate::api::stream_link::base_domain).collect();
    for u in std::iter::once(&media.hls).chain(media.subs.iter().map(|s| &s.1)) {
        if let Ok(u) = Url::parse(u) {
            out.extend(crate::api::stream_link::base_domain(&u));
        }
    }
    out.extend(crate::api::stream_link::base_domain(page));
    out.sort();
    out.dedup();
    out.truncate(12);
    out
}

/// Р Р°Р·СЂРµС€РёС‚СЊ СЃС‚СЂР°РЅРёС†Сѓ РєР°С‚Р°Р»РѕРіР° РІ РїРѕС‚РѕРє: СЃРµСЂРёРё - РїРѕ `#nova-season/episode`, С„РёР»СЊРјС‹ - `source`.
pub async fn resolve(page_url: &Url) -> anyhow::Result<(Media, Vec<String>)> {
    let (base, html) = crate::api::stream_link::page_with_url(page_url.as_str()).await?;
    let target = episode_target(page_url);
    let frames = crate::api::stream_link::find_iframes(&html, &base);
    for frame in frames.iter().take(4) {
        let Ok(ehtml) = crate::api::stream_link::page(frame.as_str()).await else { continue };
        if let Some((s, e)) = target {
            if let Some((_, _, m)) = episodes(&ehtml).into_iter().find(|(ps, pe, _)| (*ps, *pe) == (s, e)) {
                return Ok((m, extra_hosts(&base, &m, &frames)));
            }
        } else if let Some(m) = single(&ehtml) {
            return Ok((m, extra_hosts(&base, &m, &frames)));
        }
    }
    if target.is_some() {
        anyhow::bail!("РїР»РµРµСЂ РЅРµ РѕС‚РґР°Р» РІС‹Р±СЂР°РЅРЅСѓСЋ СЃРµСЂРёСЋ")
    } else {
        anyhow::bail!("РЅР° СЃС‚СЂР°РЅРёС†Рµ РЅРµ РЅР°С€Р»РѕСЃСЊ РІРёРґРµРѕ")
    }
}

/// РЎРїРёСЃРѕРє СЃРµСЂРёР№ СЃС‚СЂР°РЅРёС†С‹ РґР»СЏ РёРјРїРѕСЂС‚Р° В«СЃРµР·РѕРЅ С†РµР»РёРєРѕРјВ» (РєР°Рє `lordfilm::episodes_on_page`).
pub async fn episodes_on_page(page_url: &Url) -> anyhow::Result<Vec<(i64, i64)>> {
    let mut u = page_url.clone();
    u.set_fragment(None);
    let (base, html) = crate::api::stream_link::page_with_url(u.as_str()).await?;
    for frame in crate::api::stream_link::find_iframes(&html, &base).iter().take(4) {
        if let Ok(ehtml) = crate::api::stream_link::page(frame.as_str()).await {
            let list: Vec<(i64, i64)> = episodes(&ehtml).into_iter().map(|(s, e, _)| (s, e)).collect();
            if !list.is_empty() {
                return Ok(list);
            }
        }
    }
    anyhow::bail!("РЎРїРёСЃРѕРє РґРѕСЃС‚СѓРїРЅС‹С… СЃРµСЂРёР№ РЅРµ РЅР°Р№РґРµРЅ")
}

#[allow(dead_code)]
fn _unused(_: Duration) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_series_playlist_with_audio_and_subs() {
        let html = r#"x {"seasons":[{"season":1,"episodes":[{"episode":"1","hls":"https://cdn.example.com/a/master.m3u8?x=1","audio":{"names":["Р СѓСЃ. Р”СѓР±Р»СЏР¶","Eng.Original"],"order":[0,1]},"cc":[{"url":"https://cdn.example.com/s.vtt","name":"Р СѓСЃ. РїРѕР»РЅС‹Рµ"}],"duration":1900},{"episode":"2","hls":"https://cdn.example.com/b/master.m3u8","blocked":true}]}]}"#;
        let all = episodes(html);
        assert_eq!(all.len(), 1);
        let (s, e, m) = &all[0];
        assert_eq!((*s, *e), (1, 1));
        assert_eq!(m.audio, vec!["Р СѓСЃ. Р”СѓР±Р»СЏР¶".to_string(), "Eng.Original".to_string()]);
        assert_eq!(m.subs.len(), 1);
        assert_eq!(m.duration, Some(1900));
        assert!(valid_hls(&m.hls));
    }

    #[test]
    fn parses_movie_source_object() {
        let html = r#"makePlayer({ title: "Р”СЋРЅР°", source: { dash: "https://cdn.example.com/x.mpd", hls: "https://cdn.example.com/x.mp4/master.m3u8?a=1&b=2", audio: {"names":["JASKIER","Eng.Original"],"order":[0,1]}, cc: [{"url":"https://cdn.example.com/c.vtt","name":"Eng. SDH"}], duration: 155 } })"#;
        let m = single(html).unwrap();
        assert!(m.hls.contains("master.m3u8"));
        assert_eq!(m.audio, vec!["JASKIER".to_string(), "Eng.Original".to_string()]);
        assert_eq!(m.subs[0].1, "https://cdn.example.com/c.vtt");
        assert_eq!(m.duration, Some(155));
    }

    #[test]
    fn episode_target_fragment() {
        let u = Url::parse("https://site.example/film/x/#nova-season=2&episode=5").unwrap();
        assert_eq!(episode_target(&u), Some((2, 5)));
        let u = Url::parse("https://site.example/film/x/").unwrap();
        assert_eq!(episode_target(&u), None);
    }
}
