#!/usr/bin/env python3
"""Gera views Bronze e Silver para o DuckDB a partir dos ponteiros vigentes.

Para cada tabela lógica (``fonte`` + ``recurso`` sem as partições), junta os
Parquet da **versão vigente** de todas as partições e emite uma view
``bronze_<fonte>_<recurso>`` e ``silver_<fonte>_<recurso>``. Ler o ponteiro (``_ultima_versao.json``) e o
manifesto garante que o histórico (execuções antigas) não seja contado em dobro.

As colunas de partição (ex.: ``ano``) e de procedência (``data_ingestao``,
``execucao``) vêm do caminho via *hive partitioning* do DuckDB.

Saída: SQL em ``stdout``; um resumo em ``stderr`` (não entra no script).
"""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path

BRONZE = Path(os.environ.get("TODEOLHO_BRONZE_DIR", "/dados/bronze"))
SILVER = Path(os.environ.get("TODEOLHO_SILVER_DIR", "/dados/silver"))


def sanitizar(nome: str) -> str:
    """Reduz um nome a um identificador SQL seguro (alfanumérico + ``_``)."""
    return "".join(c if c.isalnum() else "_" for c in nome)


def base_recurso(recurso: str) -> str:
    """Remove os segmentos de partição (``chave=valor``) do recurso."""
    return "/".join(p for p in recurso.split("/") if "=" not in p)


def coletar(raiz: Path, camada: str, catalogo: dict[str, dict[str, str]] | None = None) -> dict[str, list[str]]:
    """Mapeia as views de uma camada para os Parquets da versão vigente."""
    tabelas: dict[str, list[str]] = {}
    if not raiz.is_dir():
        return tabelas
    for ponteiro in raiz.rglob("_ultima_versao.json"):
        try:
            manifesto_rel = json.loads(ponteiro.read_text(encoding="utf-8"))["manifesto"]
            manifesto = json.loads((raiz / manifesto_rel).read_text(encoding="utf-8"))
        except (OSError, ValueError, KeyError):
            continue
        fonte = manifesto.get("fonte", "")
        recurso = base_recurso(manifesto.get("recurso", ""))
        if not fonte or not recurso:
            continue
        nome = sanitizar(f"{camada}_{fonte}_{recurso}")
        if catalogo is not None:
            catalogo[nome] = {"camada": camada, "fonte": fonte}
        for parte in manifesto.get("partes", []):
            chave = parte.get("chave")
            if chave:
                tabelas.setdefault(nome, []).append(str(raiz / chave))
    return tabelas


def main() -> int:
    catalogo: dict[str, dict[str, str]] = {}
    tabelas = coletar(BRONZE, "bronze", catalogo) | coletar(SILVER, "silver", catalogo)
    if sys.argv[1:] == ["--catalogo-json"]:
        print(json.dumps([{"nome": nome, **catalogo[nome]} for nome in sorted(catalogo)]))
        return 0
    for nome in sorted(tabelas):
        arquivos = sorted(set(tabelas[nome]))
        lista = ", ".join("'" + a.replace("'", "''") + "'" for a in arquivos)
        print(f"CREATE OR REPLACE VIEW {nome} AS")
        print(
            f"  SELECT * FROM read_parquet([{lista}], "
            "union_by_name = true, hive_partitioning = true);"
        )
    print(f"-- {len(tabelas)} view(s) Bronze/Silver geradas", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
