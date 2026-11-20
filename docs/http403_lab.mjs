import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';

// Intentionally loopback-only: this lab never contacts external websites.
const host = '127.0.0.1';
const port = Number(process.env.NOVA_LAB_PORT || 8765);
const token = process.env.NOVA_LAB_TOKEN || 'student-demo-token';
const page = await readFile(new URL('./http403_lab.html', import.meta.url));

const server = createServer((req, res) => {
  const url = new URL(req.url || '/', `http://${host}:${port}`);
  const send = (status, message, details = {}) => {
    const body = JSON.stringify({ status, message, ...details });
    res.writeHead(status, { 'content-type': 'application/json; charset=utf-8', 'cache-control': 'no-store' });
    res.end(body);
    console.log(`${req.socket.remoteAddress} ${req.method} ${url.pathname} -> ${status}`);
  };

  if (req.method !== 'GET') return send(405, 'Use GET in this demonstration.');
  if (url.pathname === '/') {
    res.writeHead(200, { 'content-type': 'text/html; charset=utf-8', 'cache-control': 'no-store' });
    return res.end(page);
  }
  if (url.pathname === '/health' || url.pathname === '/public') return send(200, 'Доступ открыт.');
  if (url.pathname === '/protected') {
    if (req.headers.authorization !== `Bearer ${token}`) {
      res.writeHead(401, { 'www-authenticate': 'Bearer realm="nova-lab"', 'content-type': 'application/json; charset=utf-8' });
      return res.end(JSON.stringify({ status: 401, message: 'Нужна выданная сервером учётная запись.' }));
    }
    return send(200, 'Учётные данные приняты.');
  }
  if (url.pathname === '/forbidden') {
    const attempted = {
      'user-agent': 'Попытка подменить User-Agent отклонена политикой сервера.',
      'forwarded-for': 'Недоверенный X-Forwarded-For не меняет сетевой адрес клиента.',
      retry: 'Повтор запроса не меняет решение политики.',
    }[url.searchParams.get('demo')] || 'Ресурс запрещён политикой лабораторного сервера.';
    return send(403, attempted, {
      retryable: false,
    });
  }
  send(404, 'Маршрут не найден.');
});

server.listen(port, host, () => console.log(`Лаборатория HTTP 401/403: http://${host}:${port}`));
server.on('error', (error) => { console.error(error.message); process.exitCode = 1; });

process.on('SIGINT', () => server.close(() => process.exit(0)));
process.on('SIGTERM', () => server.close(() => process.exit(0)));
