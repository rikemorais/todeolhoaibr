#!/usr/bin/env python3
"""Console SQL web do bronze: um editor simples que roda queries no DuckDB nativo.

Serve uma página com editor + lista de tabelas e executa o SQL chamando o `duckdb`
(CLI, `-json`) sobre as views do bronze geradas em `/tmp/init.sql`. Sem dependências
externas e sem nuvem — é a alternativa self-hosted à UI hospedada da MotherDuck (#167).
"""

from __future__ import annotations

import json
import os
import subprocess
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

PORTA = int(os.environ.get("TODEOLHO_CONSULTA_PORTA_INTERNA", "4213"))
LIMITE_SQL = 200_000  # tamanho máximo de uma query, por segurança
TEMPO_LIMITE = 180  # segundos por query
MEMORIA = os.environ.get("TODEOLHO_DUCKDB_MEMORIA", "384MB")
THREADS = os.environ.get("TODEOLHO_DUCKDB_THREADS", "2")
# Ambiente do DuckDB: sem cor (ANSI) para o stderr sair limpo nos erros.
AMBIENTE = {**os.environ, "NO_COLOR": "1"}

# Prefixo injetado antes de cada query: limites + as views do bronze (por filesystem,
# sem segredo S3, para não emitir nada que suje a saída JSON). Recriado sob demanda.
_prefixo = ""


def construir_prefixo() -> str:
    """Monta o prefixo SQL (SET + CREATE VIEW do bronze) lido pelo DuckDB antes da query."""
    views = subprocess.run(
        ["python3", "/app/gerar_views.py"],
        capture_output=True,
        text=True,
        check=False,
    ).stdout
    return f"SET memory_limit = '{MEMORIA}';\nSET threads = {THREADS};\n{views}\n"

PAGINA = """<!doctype html>
<html lang="pt-BR">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>Console do bronze</title>
<style>
:root {
  color-scheme: light dark;
  --fundo: #fff; --fundo-2: #f5f6f8; --borda: #e2e5ea; --texto: #1b1f24;
  --texto-2: #5b636e; --acento: #0f766e; --erro: #b42318; --acento-fundo: #0f766e14;
}
@media (prefers-color-scheme: dark) {
  :root { --fundo: #14181d; --fundo-2: #1b2127; --borda: #2a313a; --texto: #e6e9ee;
    --texto-2: #9aa4b1; --acento: #2dd4bf; --erro: #ff7b72; --acento-fundo: #2dd4bf1f; }
}
* { box-sizing: border-box; }
body { margin: 0; font: 14px/1.5 system-ui, sans-serif; color: var(--texto);
  background: var(--fundo); height: 100vh; display: flex; flex-direction: column; }
header { padding: 10px 16px; border-bottom: 1px solid var(--borda);
  display: flex; align-items: center; gap: 10px; flex: none; }
header h1 { font-size: 15px; margin: 0; font-weight: 700; }
header .sub { color: var(--texto-2); font-size: 12px; }
main { flex: 1; display: flex; min-height: 0; }
aside { width: 240px; border-right: 1px solid var(--borda); overflow: auto;
  padding: 10px; flex: none; background: var(--fundo-2); }
aside h2 { font-size: 11px; text-transform: uppercase; letter-spacing: .05em;
  color: var(--texto-2); margin: 4px 6px 8px; }
aside button { display: block; width: 100%; text-align: left; border: 0; background: none;
  color: var(--texto); padding: 5px 6px; border-radius: 6px; cursor: pointer;
  font: inherit; word-break: break-all; }
aside button:hover { background: var(--acento-fundo); color: var(--acento); }
section { flex: 1; display: flex; flex-direction: column; min-width: 0; }
.editor { padding: 12px 16px; border-bottom: 1px solid var(--borda); }
textarea { width: 100%; height: 120px; resize: vertical; font-family: ui-monospace, monospace;
  font-size: 13px; padding: 10px; border: 1px solid var(--borda); border-radius: 8px;
  background: var(--fundo); color: var(--texto); }
.acoes { margin-top: 8px; display: flex; gap: 10px; align-items: center; }
button.primario { background: var(--acento); color: #fff; border: 0; padding: 7px 16px;
  border-radius: 8px; font-weight: 600; cursor: pointer; }
button.primario:disabled { opacity: .5; cursor: default; }
.dica { color: var(--texto-2); font-size: 12px; }
.resultado { flex: 1; overflow: auto; padding: 0 16px 16px; }
table { border-collapse: collapse; width: 100%; font-size: 13px; }
th, td { border: 1px solid var(--borda); padding: 4px 8px; text-align: left;
  white-space: pre-wrap; vertical-align: top; max-width: 420px; }
th { position: sticky; top: 0; background: var(--fundo-2); font-weight: 600; }
td.nulo { color: var(--texto-2); font-style: italic; }
.msg { padding: 12px 16px; color: var(--texto-2); }
.msg.erro { color: var(--erro); font-family: ui-monospace, monospace; white-space: pre-wrap; }
</style>
</head>
<body>
<header>
  <h1>Console do bronze</h1>
  <span class="sub">DuckDB sobre o Parquet, somente leitura</span>
</header>
<main>
  <aside>
    <h2>Tabelas</h2>
    <div id="tabelas"><span class="dica">carregando…</span></div>
  </aside>
  <section>
    <div class="editor">
      <textarea id="sql" spellcheck="false"
        placeholder="SELECT * FROM bronze_camara_despesas LIMIT 100;"></textarea>
      <div class="acoes">
        <button class="primario" id="rodar">Rodar (Ctrl+Enter)</button>
        <span class="dica" id="status"></span>
      </div>
    </div>
    <div class="resultado" id="resultado">
      <p class="msg">Escolha uma tabela à esquerda ou escreva uma query e rode.</p>
    </div>
  </section>
</main>
<script>
const $ = (s) => document.querySelector(s);
const sql = $("#sql"), status = $("#status"), resultado = $("#resultado");

async function carregarTabelas() {
  try {
    const r = await fetch("api/tabelas");
    const d = await r.json();
    const div = $("#tabelas");
    div.innerHTML = "";
    if (!d.tabelas || !d.tabelas.length) {
      div.innerHTML = '<span class="dica">nenhuma tabela</span>';
      return;
    }
    for (const t of d.tabelas) {
      const b = document.createElement("button");
      b.textContent = t;
      b.onclick = () => { sql.value = `SELECT * FROM ${t} LIMIT 100;`; rodar(); };
      div.appendChild(b);
    }
  } catch (e) { $("#tabelas").innerHTML = '<span class="dica">falha ao listar</span>'; }
}

function celula(v) {
  const td = document.createElement("td");
  if (v === null) { td.textContent = "NULL"; td.className = "nulo"; }
  else td.textContent = String(v);
  return td;
}

function mostrarTabela(linhas) {
  if (!linhas.length) { resultado.innerHTML = '<p class="msg">0 linhas.</p>'; return; }
  const cols = Object.keys(linhas[0]);
  const tabela = document.createElement("table");
  const thead = tabela.createTHead().insertRow();
  for (const c of cols) { const th = document.createElement("th"); th.textContent = c; thead.appendChild(th); }
  const tbody = tabela.createTBody();
  for (const linha of linhas) {
    const tr = tbody.insertRow();
    for (const c of cols) tr.appendChild(celula(linha[c]));
  }
  resultado.innerHTML = "";
  resultado.appendChild(tabela);
}

async function rodar() {
  const texto = sql.value.trim();
  if (!texto) return;
  $("#rodar").disabled = true;
  status.textContent = "rodando…";
  const inicio = performance.now();
  try {
    const r = await fetch("api/query", { method: "POST", body: texto });
    const d = await r.json();
    const ms = Math.round(performance.now() - inicio);
    if (d.erro) {
      resultado.innerHTML = "";
      const p = document.createElement("p");
      p.className = "msg erro"; p.textContent = d.erro;
      resultado.appendChild(p);
      status.textContent = `erro (${ms} ms)`;
    } else {
      mostrarTabela(d.linhas || []);
      status.textContent = `${(d.linhas || []).length} linha(s) · ${ms} ms`;
    }
  } catch (e) {
    resultado.innerHTML = '<p class="msg erro">Falha ao falar com o servidor.</p>';
    status.textContent = "erro";
  } finally { $("#rodar").disabled = false; }
}

$("#rodar").onclick = rodar;
sql.addEventListener("keydown", (e) => {
  if ((e.ctrlKey || e.metaKey) && e.key === "Enter") { e.preventDefault(); rodar(); }
});
carregarTabelas();
</script>
</body>
</html>
"""


def _limpar_erro(texto: str) -> str:
    """Remove linhas de ruído do DuckDB (avisos e dicas) da mensagem de erro."""
    linhas = [
        linha
        for linha in texto.splitlines()
        if not linha.startswith("-- Loading resources")
        and not linha.startswith("hint:")
    ]
    return "\n".join(linhas).strip()


def rodar_sql(sql: str) -> dict:
    """Executa o SQL no DuckDB (prefixo + query, via stdin) e devolve {linhas} ou {erro}."""
    if len(sql) > LIMITE_SQL:
        return {"erro": "query grande demais."}
    programa = f"{_prefixo}\n{sql}"
    try:
        proc = subprocess.run(
            ["duckdb", "-json"],
            input=programa,
            capture_output=True,
            text=True,
            timeout=TEMPO_LIMITE,
            env=AMBIENTE,
        )
    except subprocess.TimeoutExpired:
        return {"erro": f"a query passou de {TEMPO_LIMITE}s e foi interrompida."}
    # A query da vez roda depois das views do prefixo. Se alguma view falhar ao ser criada
    # (ex.: um Parquet vazio de uma fonte sem dados num período), o DuckDB segue e ainda
    # executa a query — então, se houver saída JSON válida, usamos, mesmo com exit != 0.
    saida = proc.stdout.strip()
    if saida:
        try:
            dados = json.loads(saida)
            return {"linhas": dados if isinstance(dados, list) else [dados]}
        except json.JSONDecodeError:
            pass
    if proc.returncode != 0:
        return {"erro": _limpar_erro(proc.stderr or saida) or "erro desconhecido"}
    return {"linhas": []}


def listar_tabelas() -> list[str]:
    # Recria as views a cada listagem (ao carregar/atualizar a página): assim fontes
    # refinadas depois que o console subiu aparecem sem precisar reiniciar o serviço.
    global _prefixo
    _prefixo = construir_prefixo()
    resultado = rodar_sql(
        "SELECT table_name FROM information_schema.tables "
        "WHERE table_schema = 'main' ORDER BY table_name;"
    )
    return [linha["table_name"] for linha in resultado.get("linhas", [])]


class Manipulador(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def _responder(self, codigo: int, corpo: bytes, tipo: str) -> None:
        self.send_response(codigo)
        self.send_header("Content-Type", tipo)
        self.send_header("Content-Length", str(len(corpo)))
        self.send_header("Content-Security-Policy", "default-src 'none'; style-src 'unsafe-inline'; "
                         "script-src 'unsafe-inline'; connect-src 'self'")
        self.send_header("X-Content-Type-Options", "nosniff")
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(corpo)

    def _json(self, dados: dict) -> None:
        self._responder(200, json.dumps(dados).encode(), "application/json")

    def do_GET(self) -> None:
        if self.path == "/" or self.path.startswith("/?"):
            self._responder(200, PAGINA.encode(), "text/html; charset=utf-8")
        elif self.path == "/api/tabelas":
            self._json({"tabelas": listar_tabelas()})
        else:
            self._responder(404, b"nao encontrado", "text/plain")

    def do_POST(self) -> None:
        if self.path != "/api/query":
            self._responder(404, b"nao encontrado", "text/plain")
            return
        tamanho = int(self.headers.get("Content-Length", "0"))
        sql = self.rfile.read(tamanho).decode("utf-8", "replace")
        self._json(rodar_sql(sql))

    def log_message(self, *_args) -> None:  # silencia o log por requisição
        pass


def main() -> None:
    global _prefixo
    _prefixo = construir_prefixo()
    servidor = ThreadingHTTPServer(("0.0.0.0", PORTA), Manipulador)
    print(f"Console do bronze em http://localhost:{PORTA}", flush=True)
    servidor.serve_forever()


if __name__ == "__main__":
    main()
