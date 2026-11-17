//! Проверка сайта самим хостом в отдельном WebView, без IPC-доступа к Nova.
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub fn open(app: &AppHandle, id: &str) -> Result<(), String> {
    let source = fsfy::site_sources::by_id(id).ok_or("Неизвестный источник")?;
    let label = format!("source-{}", source.id);
    if let Some(window) = app.get_webview_window(&label) {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }
    let directory = app.path().app_data_dir().map_err(|e| e.to_string())?.join("source-sessions").join(source.id);
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    WebviewWindowBuilder::new(
        app,
        label,
        WebviewUrl::External(source.url.parse().map_err(|_| "Некорректный адрес источника")?),
    )
    .title(format!("Nova - проверка источника {}", source.name))
    .user_agent(fsfy::site_sources::USER_AGENT)
    .inner_size(1100.0, 780.0)
    .data_directory(directory)
    .build()
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn cookies(app: &AppHandle, id: &str) -> Result<Vec<(String, String)>, String> {
    let source = fsfy::site_sources::by_id(id).ok_or("Неизвестный источник")?;
    let window = app
        .get_webview_window(&format!("source-{}", source.id))
        .ok_or("Откройте окно проверки и пройдите её вручную")?;
    let url = source.url.parse().map_err(|_| "Некорректный адрес источника")?;
    window
        .cookies_for_url(url)
        .map(|cookies| cookies.into_iter().map(|c| (c.name().to_string(), c.value().to_string())).collect())
        .map_err(|e| e.to_string())
}
