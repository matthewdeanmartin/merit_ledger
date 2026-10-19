// Dev-server proxy: the API and the board's own pages go to a Merit Ledger
// server, so the browser only talks to localhost:4200 (same origin).
//
//   npm start                                   -> desktop server on :8089
//   MERITLEDGER=192.168.1.170 npm start         -> a board, plain HTTP
const raw = (process.env['MERITLEDGER'] ?? 'localhost:8089').trim();
const target = /^https?:\/\//i.test(raw) ? raw : `http://${raw}`;
console.log(`[proxy] /api, /trust, /ca -> ${target}`);

const route = { target, secure: false, changeOrigin: true, proxyTimeout: 30_000, timeout: 30_000 };

export default { '/api': route, '/trust': route, '/ca': route, '/metrics': route };
