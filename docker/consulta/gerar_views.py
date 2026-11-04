#!/usr/bin/env python3
"""Gera as views do bronze para o DuckDB a partir dos ponteiros de versão vigente.

Para cada tabela lógica (``fonte`` + ``recurso`` sem as partições), junta os
Parquet da **versão vigente** de todas as partições e emite uma view
``bronze_<fonte>_<recurso>``. Ler o ponteiro (``_ultima_versao.json``) e o
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


def sanitizar(nome: str) -> str:
    """Reduz um nome a um identificador SQL seguro (alfanumérico + ``_``)."""
    return "".join(c if c.isalnum() else "_" for c in nome)


def base_recurso(recurso: str) -> str:
    """Remove os segmentos de partição (``chave=valor``) do recurso."""
    return "/".join(p for p in recurso.split("/") if "=" not in p)


def coletar() -> dict[str, list[str]]:
    """Mapeia cada view do bronze para os Parquet da versão vigente."""
    tabelas: dict[str, list[str]] = {}
    if not BRONZE.is_dir():
        return tabelas
    for ponteiro in BRONZE.rglob("_ultima_versao.json"):
        try:
            manifesto_rel = json.loads(ponteiro.read_text(encoding="utf-8"))["manifesto"]
            manifesto = json.loads((BRONZE / manifesto_rel).read_text(encoding="utf-8"))
        except (OSError, ValueError, KeyError):
            continue
        fonte = manifesto.get("fonte", "")
        recurso = base_recurso(manifesto.get("recurso", ""))
        if not fonte or not recurso:
            continue
        nome = sanitizar(f"bronze_{fonte}_{recurso}")
        for parte in manifesto.get("partes", []):
            chave = parte.get("chave")
            if chave:
                tabelas.setdefault(nome, []).append(str(BRONZE / chave))
    return tabelas


def main() -> int:
    tabelas = coletar()
    for nome in sorted(tabelas):
        arquivos = sorted(set(tabelas[nome]))
        lista = ", ".join("'" + a.replace("'", "''") + "'" for a in arquivos)
        print(f"CREATE OR REPLACE VIEW {nome} AS")
        print(
            f"  SELECT * FROM read_parquet([{lista}], "
            "union_by_name = true, hive_partitioning = true);"
        )
    print(f"-- {len(tabelas)} view(s) de bronze geradas", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
