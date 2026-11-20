use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

const PAGE: &str = include_str!("../index.html");

fn respond(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8]) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'self'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)
}

fn json(status: u16, title: &str, decision: &str, explanation: &str, simulated: bool) -> Vec<u8> {
    // The strings passed here are fixed training text, never user-supplied input.
    format!(
        "{{\"status\":{status},\"title\":\"{title}\",\"decision\":\"{decision}\",\"explanation\":\"{explanation}\",\"simulated\":{simulated}}}"
    )
    .into_bytes()
}

fn handle(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buffer = [0_u8; 8192];
    let size = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..size]);
    let path = request.lines().next().and_then(|line| line.split_whitespace().nth(1)).unwrap_or("/");

    if path == "/" || path == "/index.html" {
        return respond(&mut stream, "200 OK", "text/html; charset=utf-8", PAGE.as_bytes());
    }

    if let Some((_, query)) = path.split_once("/api/demo?") {
        let scenario = query.split('&').find_map(|part| part.strip_prefix("scenario=")).unwrap_or("");
        let (status, title, decision, explanation) = match scenario {
            "baseline" => (403, "Обычный запрос", "Отказано по политике", "Доступ не выдан: корректный сервер отвечает 403."),
            "unsafe-header" => (200, "Уязвимая модель", "Ложный пропуск только в симуляции", "Учебная модель ошибочно доверяет клиентским метаданным. Реальный внешний запрос не выполнялся."),
            "safe-proxy" => (403, "Исправленная модель", "Отказано корректно", "Решение основано на соединении и доверенной конфигурации сервера, а не на произвольном вводе клиента."),
            "user-agent" => (403, "Подмена User-Agent", "Доступ не изменился", "Заголовок клиента не является разрешением."),
            "retry" => (403, "Повтор запроса", "Доступ не изменился", "Повтор не меняет политику доступа."),
            _ => return respond(&mut stream, "404 Not Found", "application/json", b"{\"error\":\"unknown scenario\"}"),
        };
        return respond(&mut stream, "200 OK", "application/json; charset=utf-8", &json(status, title, decision, explanation, true));
    }

    if let Some((_, query)) = path.split_once("/api/source?") {
        let name = query.split('&').find_map(|part| part.strip_prefix("name=")).unwrap_or("");
        let title = match name {
            "lordfilm" => "LordFilm · фиктивный учебный кейс",
            "lordserials" => "Lordserials · фиктивный учебный кейс",
            "yummyanime" => "YummyAnime · фиктивный учебный кейс",
            _ => return respond(&mut stream, "404 Not Found", "application/json", b"{\"error\":\"unknown case\"}"),
        };
        return respond(&mut stream, "200 OK", "application/json; charset=utf-8", &json(
            403,
            title,
            "Имитация отказа доступа",
            "Это заранее заданный пример. Лаборатория не соединялась с указанным сайтом и не проверяла его реальный статус.",
            true,
        ));
    }

    respond(&mut stream, "404 Not Found", "text/plain; charset=utf-8", b"Not found\n")
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", 8765))?;
    println!("Nova: учебная лаборатория HTTP 401/403");
    println!("Откройте http://127.0.0.1:8765 в браузере. Остановка: Ctrl+C.");
    println!("Все сценарии имитируются локально; внешние сайты не запрашиваются.");
    for connection in listener.incoming() {
        match connection {
            Ok(stream) => { let _ = handle(stream); }
            Err(error) => eprintln!("Ошибка локального соединения: {error}"),
        }
    }
    Ok(())
}
