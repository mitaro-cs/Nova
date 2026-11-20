//! Лёгкая проверка «Я не робот» на сайтах-каталогах (Antibot Cloud).
//!
//! Это не капча с картинками: сайт отдаёт 403 и страницу со скриптом. Браузер считает
//! данные (`data`), нажимает видимый блок «Я не робот», сервер возвращает cookie, после
//! чего страница открывается обычно. Повторяем тот же честный обмен, чтобы сервер мог
//! скачать страницу так же, как браузер зрителя. Никакие платные или скрытые сервисы
//! не используются.

use std::{collections::HashMap, sync::OnceLock, time::Duration};

use anyhow::Context;
use base64::Engine;
use parking_lot::Mutex;
use regex::Regex;
use reqwest::{Url, header};

/// Куки живут недолго; храним их по хосту, чтобы не проходить проверку на каждую серию.
const TTL_MS: i64 = 30 * 60_000;
type Jar = Mutex<HashMap<String, (i64, Vec<(String, String)>)>>;

fn jar() -> &'static Jar {
    static J: OnceLock<Jar> = OnceLock::new();
    J.get_or_init(Mutex::default)
}

/// Признак именно той проверки, которую мы умеем проходить.
pub fn is_challenge(body: &str) -> bool {
    body.contains("/antibot8/index.php") && body.contains("b64_to_utf8(")
}

/// Ранее полученные куки для этого хоста (если не истекли).
pub fn saved(host: &str) -> Vec<(String, String)> {
    let mut jar = jar().lock();
    let Some((at, values)) = jar.get(host) else { return Vec::new() };
    if crate::util::now_ms() - *at > TTL_MS {
        jar.remove(host);
        return Vec::new();
    }
    values.clone()
}

pub fn remember(host: &str, pairs: &[(String, String)]) {
    if pairs.is_empty() {
        return;
    }
    jar().lock().insert(host.to_string(), (crate::util::now_ms(), pairs.to_vec()));
}

/// Повторяет обмен браузера и возвращает куки, которые нужно добавить в запрос.
pub async fn solve(
    client: &reqwest::Client,
    url: &Url,
    body: &str,
    cookies: &HashMap<String, String>,
) -> anyhow::Result<Vec<(String, String)>> {
    let data = challenge_data(body).context("нет данных проверки")?;
    let token = visible_token(body).context("нет видимой кнопки «Я не робот»")?;
    let name = Regex::new(r#"document\.cookie\s*=\s*"([^"]+)"\s*\+\s*obj\.cookie"#)?
        .captures(body)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim_end_matches('=').to_string())
        .context("нет имени cookie проверки")?;
    let endpoint = url.join("/antibot8/index.php")?;
    let jar = cookies.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("; ");

    // Браузер делает такой запрос первым: он включает «серый» режим и не мешает основному.
    post(client, &endpoint, url, &jar, &format!("{data}&x52a087d1c5a43b9a83c5ba643b52c683=ab&xxx=&rowid=0&gray=1"))
        .await?;

    let answer = post(
        client,
        &endpoint,
        url,
        &jar,
        &format!("{data}&x52a087d1c5a43b9a83c5ba643b52c683=post&xxx={token}&rowid=0&gray=1"),
    )
    .await?;
    let value: serde_json::Value = serde_json::from_str(&answer).context("проверка вернула не JSON")?;
    let value = value["cookie"].as_str().context("проверка не вернула cookie")?;
    let date = Regex::new(r"date=(\d+)")?.captures(&data).and_then(|c| c.get(1)).map_or("", |m| m.as_str());
    Ok(vec![(name, format!("{value}-{date}"))])
}

async fn post(client: &reqwest::Client, endpoint: &Url, page: &Url, jar: &str, body: &str) -> anyhow::Result<String> {
    let response = crate::api::stream_link::browser_document(
        client
            .post(endpoint.clone())
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded;")
            .header(header::REFERER, page.as_str())
            .header(header::ORIGIN, page.origin().ascii_serialization())
            .header(header::COOKIE, jar),
        false,
    )
    .body(body.to_string())
    .timeout(Duration::from_secs(10))
    .send()
    .await?;
    anyhow::ensure!(response.status().is_success(), "проверка ответила {}", response.status().as_u16());
    crate::outbound::limited(response, 64 * 1024).await.map(|v| String::from_utf8_lossy(&v).into_owned())
}

/// `data = '…'+adb+'…'+screen.width+…;` - собираем строку, подставляя известные браузерные значения.
fn challenge_data(body: &str) -> Option<String> {
    let at = body.find("data = '")? + "data = '".len();
    let rest = &body[at..];
    let end = rest.find('\'')?;
    let mut out = rest[..end].to_string();
    let tail = &rest[end + 1..];
    let stop = tail.find(';').unwrap_or(tail.len());
    for (i, part) in tail[..stop].split('+').enumerate() {
        let part = part.trim();
        if i % 2 == 0 {
            out.push_str(part.trim_matches(|c| c == '\'' || c == '"'));
        } else {
            out.push_str(expand(part));
        }
    }
    (!out.is_empty()).then_some(out)
}

fn expand(expr: &str) -> &'static str {
    match expr {
        "adb" => "1",
        "screen.width" => "1920",
        "screen.height" => "1080",
        "document.documentElement.clientWidth" => "1000",
        "document.documentElement.clientHeight" => "700",
        "screen.colorDepth" | "screen.pixelDepth" => "24",
        "encodeURIComponent(document.referrer)" => "",
        "Intl.DateTimeFormat().resolvedOptions().timeZone" => "Europe/Moscow",
        "ipdbc" | "ipv4" | "rct" => "",
        "cookieoff" => "0",
        _ => "",
    }
}

/// На странице несколько одинаковых кнопок: одна видимая, остальные спрятаны стилем.
fn visible_token(body: &str) -> Option<String> {
    let html = Regex::new(r#"b64_to_utf8\("([A-Za-z0-9+/=]+)"\)"#).ok()?.captures(body)?.get(1)?.as_str();
    let bytes = base64::engine::general_purpose::STANDARD.decode(html).ok()?;
    let html = String::from_utf8_lossy(&bytes);
    let hidden: Vec<String> = Regex::new(r"\.([A-Za-z0-9_-]+)\s*\{\s*display:\s*none")
        .ok()?
        .captures_iter(&html)
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .collect();
    let re = Regex::new(r#"class="([^"]+)"[^>]*onclick="[A-Za-z0-9_]+\('post',\s*data,\s*'([^']+)'\)""#).ok()?;
    let found: Vec<(String, String)> = re
        .captures_iter(&html)
        .filter_map(|c| Some((c.get(1)?.as_str().to_string(), c.get(2)?.as_str().to_string())))
        .collect();
    found
        .iter()
        .find(|(class, _)| !class.split_whitespace().any(|c| hidden.iter().any(|h| h == c)))
        .or_else(|| found.first())
        .map(|(_, token)| token.clone())
        .filter(|t| !t.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    const BODY: &str = r#"
      <script>
      data = 'useragent=x&date=1795035331&country=RU'+adb+'&w='+screen.width+'&h='+screen.height+'&tz='+Intl.DateTimeFormat().resolvedOptions().timeZone+'&ipdbc='+ipdbc+'&cookieoff='+cookieoff;
      document.cookie = "SESSION=" + obj.cookie + "-1795035331; SameSite=Lax";
      </script>
    "#;

    #[test]
    fn разбирает_данные_проверки() {
        assert_eq!(
            challenge_data(BODY).as_deref(),
            Some("useragent=x&date=1795035331&country=RU1&w=1920&h=1080&tz=Europe/Moscow&ipdbc=&cookieoff=0")
        );
        assert_eq!(
            Regex::new(r#"document\.cookie\s*=\s*"([^"]+)"\s*\+\s*obj\.cookie"#)
                .unwrap()
                .captures(BODY)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().trim_end_matches('=')),
            Some("SESSION")
        );
    }

    #[test]
    fn отличает_проверку_от_обычной_страницы() {
        assert!(!is_challenge("<html><body>Клан Сопрано</body></html>"));
        assert!(is_challenge("/antibot8/index.php ... b64_to_utf8(\"PHA+\")"));
        assert!(visible_token("<html>нет проверки</html>").is_none());
    }
}
