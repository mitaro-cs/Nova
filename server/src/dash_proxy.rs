//! Переписывание MPD с сохранением DASH-шаблонов $Number$, $Time$, $RepresentationID$.
use base64::Engine;
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};
use reqwest::Url;

pub fn proxy_url(id: i64, url: &Url) -> String {
    let origin = url.origin().ascii_serialization();
    let key = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(origin.as_bytes());
    format!(
        "/api/links/{id}/stream/dash/{key}{}{}",
        url.path(),
        url.query().map(|q| format!("?{q}")).unwrap_or_default()
    )
}

pub fn target(origin: &str, path: &str, query: Option<&str>) -> anyhow::Result<Url> {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(origin)?;
    let root = Url::parse(std::str::from_utf8(&bytes)?)?;
    anyhow::ensure!(
        root.path() == "/" && root.query().is_none() && root.fragment().is_none() && crate::outbound::valid_url(&root),
        "Недопустимый адрес DASH"
    );
    let mut url = root.join(path)?;
    url.set_query(query);
    anyhow::ensure!(url.origin() == root.origin() && crate::outbound::valid_url(&url), "Недопустимый путь DASH");
    Ok(url)
}

pub fn rewrite(text: &str, base: &Url, id: i64) -> anyhow::Result<String> {
    anyhow::ensure!(
        !text.contains("<!DOCTYPE") && !text.contains("<!ENTITY") && !text.contains("xlink:href"),
        "Внешние XML-сущности и XLink в MPD не поддерживаются"
    );
    let mut scan = Reader::from_str(text);
    let mut depth = 0;
    let mut root_base = false;
    loop {
        match scan.read_event()? {
            Event::Start(e) => {
                depth += 1;
                if e.local_name().as_ref() == "BaseURL" && depth == 2 {
                    root_base = true
                }
            }
            Event::End(_) => depth -= 1,
            Event::Eof => break,
            _ => {}
        }
    }
    let mut reader = Reader::from_str(text);
    let mut writer = Writer::new(Vec::new());
    let mut base_depth = None;
    depth = 0;
    loop {
        match reader.read_event()? {
            Event::Start(e) => {
                depth += 1;
                let name = e.local_name();
                if name.as_ref() == "BaseURL" {
                    base_depth = Some(depth);
                }
                let root = name.as_ref() == "MPD";
                writer.write_event(Event::Start(rewrite_attributes(e, base, id)?))?;
                if root && !root_base {
                    let directory = base.join(".")?;
                    writer.write_event(Event::Start(BytesStart::new("BaseURL")))?;
                    writer.write_event(Event::Text(BytesText::new(&proxy_url(id, &directory))))?;
                    writer.write_event(Event::End(BytesEnd::new("BaseURL")))?;
                }
            }
            Event::Empty(e) => writer.write_event(Event::Empty(rewrite_attributes(e, base, id)?))?,
            Event::Text(e) if base_depth.is_some() => {
                let value = e.xml10_content();
                let value = quick_xml::escape::unescape(&value)?;
                let value = value.trim();
                let rewritten = if base_depth == Some(2)
                    || value.starts_with('/')
                    || value.starts_with("https:")
                    || value.starts_with("http:")
                {
                    proxy_url(id, &base.join(value)?)
                } else {
                    value.to_owned()
                };
                writer.write_event(Event::Text(BytesText::new(&rewritten)))?;
            }
            Event::End(e) => {
                if base_depth == Some(depth) {
                    base_depth = None;
                }
                depth -= 1;
                writer.write_event(Event::End(e))?;
            }
            Event::Eof => break,
            e => writer.write_event(e)?,
        }
    }
    Ok(String::from_utf8(writer.into_inner())?)
}

fn rewrite_attributes<'a>(mut e: BytesStart<'a>, base: &Url, id: i64) -> anyhow::Result<BytesStart<'a>> {
    let attributes = e
        .attributes()
        .map(|a| {
            let a = a?;
            let key = a.key.as_ref().to_owned();
            let mut value = a.normalized_value(quick_xml::XmlVersion::Implicit1_0)?.into_owned();
            if matches!(key.as_str(), "media" | "initialization" | "sourceURL" | "index")
                && (value.starts_with('/') || value.starts_with("http:") || value.starts_with("https:"))
            {
                value = proxy_url(id, &base.join(&value)?);
            }
            Ok((key, value))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    e.clear_attributes();
    for (k, v) in &attributes {
        e.push_attribute((k.as_str(), v.as_str()));
    }
    Ok(e)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_templates_and_redirects_all_segments_to_nova() {
        let base = Url::parse("https://cdn.example.com/a/b/movie.mpd").unwrap();
        let xml = r#"<MPD><Period><AdaptationSet><Representation id="v"><SegmentTemplate initialization="init-$RepresentationID$.m4s" media="chunk-$Number%05d$.m4s"/></Representation></AdaptationSet></Period></MPD>"#;
        let out = rewrite(xml, &base, 7).unwrap();
        assert!(out.contains(&proxy_url(7, &base.join(".").unwrap())));
        assert!(out.contains("chunk-$Number%05d$.m4s"));
        assert!(out.contains("init-$RepresentationID$.m4s"));
        let origin = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("https://cdn.example.com");
        assert_eq!(
            target(&origin, "a/b/init-v.m4s", Some("token=x")).unwrap().as_str(),
            "https://cdn.example.com/a/b/init-v.m4s?token=x"
        );
        assert!(target(&origin, "//evil.example/x", None).is_err());
    }
    #[test]
    fn rewrites_existing_base_and_absolute_refs_and_rejects_entities() {
        let base = Url::parse("https://cdn.example.com/a/movie.mpd").unwrap();
        let xml = r#"<MPD><BaseURL>../data/</BaseURL><Period><SegmentURL media="https://cdn.example.com/a.mp4?a=1&amp;b=2"/></Period></MPD>"#;
        let out = rewrite(xml, &base, 9).unwrap();
        assert_eq!(out.matches("<BaseURL>").count(), 1);
        assert!(!out.contains("media=\"https:"));
        assert!(out.contains("a=1&amp;b=2"));
        assert!(rewrite("<!DOCTYPE x><MPD/>", &base, 9).is_err());
    }
}
