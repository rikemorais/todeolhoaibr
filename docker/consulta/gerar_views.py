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
import subprocess
import sys
from pathlib import Path

BRONZE = Path(os.environ.get("TODEOLHO_BRONZE_DIR", "/dados/bronze"))
SILVER = Path(os.environ.get("TODEOLHO_SILVER_DIR", "/dados/silver"))

# Chaves candidatas declaradas na matriz Silver. A consulta mede duplicatas,
# mas nunca elimina linhas. Chaves ausentes ficam explicitamente sem validação.
CHAVES_CANDIDATAS: dict[tuple[str, str], tuple[str, ...]] = {
    ("ana", "estacoes-fluviometricas"): ("codigo_estacao",),
    ("ana", "estacoes-pluviometricas"): ("codigo_estacao",),
    ("mapa_empresas", "movimentacao"): ("periodo", "uf"),
    ("b3", "cotahist"): (
        "data_pregao",
        "codigo_negociacao",
        "codigo_bdi",
        "tipo_mercado",
    ),
    ("undp", "indices-compostos"): ("iso3",),
    ("world-bank", "indicadores/desemprego"): (
        "codigo_pais",
        "codigo_indicador",
        "ano",
    ),
    ("world-bank", "indicadores/alfabetizacao"): (
        "codigo_pais",
        "codigo_indicador",
        "ano",
    ),
    ("world-bank", "indicadores/expectativa-vida"): (
        "codigo_pais",
        "codigo_indicador",
        "ano",
    ),
    ("world-bank", "indicadores/gini"): (
        "codigo_pais",
        "codigo_indicador",
        "ano",
    ),
    ("world-bank", "indicadores/pib"): (
        "codigo_pais",
        "codigo_indicador",
        "ano",
    ),
    ("world-bank", "indicadores/pib-per-capita"): (
        "codigo_pais",
        "codigo_indicador",
        "ano",
    ),
    ("world-bank", "indicadores/pobreza-extrema"): (
        "codigo_pais",
        "codigo_indicador",
        "ano",
    ),
    ("world-bank", "indicadores/populacao"): (
        "codigo_pais",
        "codigo_indicador",
        "ano",
    ),
}

COLUNAS_LINHAGEM = (
    "_tdoa_origem_manifesto",
    "_tdoa_origem_parte",
    "_tdoa_origem_ordinal",
)


def sanitizar(nome: str) -> str:
    """Reduz um nome a um identificador SQL seguro (alfanumérico + ``_``)."""
    return "".join(c if c.isalnum() else "_" for c in nome)


def citar_identificador(nome: str) -> str:
    """Escapa um identificador DuckDB, inclusive nomes vindos de leiautes externos."""
    return '"' + nome.replace('"', '""') + '"'


def chave_candidata(fonte: str, recurso: str) -> tuple[str, ...] | None:
    """Resolve chaves candidatas sem inferi-las de nomes arbitrários de colunas."""
    if fonte == "bcb" and recurso.startswith("series/"):
        # O identificador da série já faz parte do recurso; cada view cobre uma série.
        return ("data",)
    return CHAVES_CANDIDATAS.get((fonte, recurso))


def base_recurso(recurso: str) -> str:
    """Remove os segmentos de partição (``chave=valor``) do recurso."""
    return "/".join(p for p in recurso.split("/") if "=" not in p)


def coletar(
    raiz: Path,
    camada: str,
    catalogo: dict[str, dict[str, str]] | None = None,
    metricas_silver: dict[str, list[dict | None]] | None = None,
) -> dict[str, list[str]]:
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
            catalogo[nome] = {
                "camada": camada,
                "fonte": fonte,
                "recurso": recurso,
            }
        tem_parte = False
        for parte in manifesto.get("partes", []):
            chave = parte.get("chave")
            if chave:
                tabelas.setdefault(nome, []).append(str(raiz / chave))
                tem_parte = True
        if camada == "silver" and metricas_silver is not None and tem_parte:
            metricas_silver.setdefault(nome, []).append(manifesto.get("qualidade"))
    return tabelas


def agregar_metricas(manifestos: list[dict | None]) -> dict | None:
    """Soma apenas métricas de todos os manifestos vigentes de uma view."""
    if not manifestos:
        return None
    totais: dict = {
        "linhas_bronze_preparadas": 0,
        "linhas_saida": 0,
        "linhas_rejeitadas": 0,
        "copias_removidas": 0,
        "rejeicoes_por_motivo": {},
        "conversoes_opcionais_por_motivo": {},
    }
    for qualidade in manifestos:
        if not isinstance(qualidade, dict) or qualidade.get("versao") != 2:
            return None
        for campo in ("linhas_bronze_preparadas", "linhas_saida", "linhas_rejeitadas", "copias_removidas"):
            valor = qualidade.get(campo)
            if type(valor) is not int or valor < 0:
                return None
            totais[campo] += valor
        for campo in ("rejeicoes_por_motivo", "conversoes_opcionais_por_motivo"):
            contagens = qualidade.get(campo)
            if not isinstance(contagens, dict):
                return None
            for motivo, quantidade in contagens.items():
                if not isinstance(motivo, str) or type(quantidade) is not int or quantidade < 0:
                    return None
                totais[campo][motivo] = totais[campo].get(motivo, 0) + quantidade
    return totais


def literal_sql(valor: str) -> str:
    return "'" + valor.replace("'", "''") + "'"


def inspecionar_colunas(arquivos: list[str]) -> list[str]:
    """Lê somente o schema Parquet para produzir expressões de qualidade."""
    lista = ", ".join("'" + arquivo.replace("'", "''") + "'" for arquivo in arquivos)
    sql = (
        "DESCRIBE SELECT * FROM read_parquet(["
        f"{lista}], union_by_name = true, hive_partitioning = true);"
    )
    processo = subprocess.run(
        ["duckdb", "-json", "-c", sql],
        capture_output=True,
        text=True,
        check=False,
    )
    if processo.returncode != 0 or not processo.stdout.strip():
        return []
    try:
        return [linha["column_name"] for linha in json.loads(processo.stdout)]
    except (json.JSONDecodeError, KeyError, TypeError):
        return []


def sql_qualidade(
    nome_view: str,
    colunas: list[str],
    chave: tuple[str, ...] | None,
    metricas: dict | None = None,
) -> str:
    """Gera uma view de perfil lazy sobre uma única tabela Silver vigente."""
    expressoes = ["count(*) AS linhas"]
    for coluna in colunas:
        expressoes.append(
            "count(*) FILTER (WHERE "
            f"{citar_identificador(coluna)} IS NULL) AS "
            f"{citar_identificador('nulos__' + coluna)}"
        )

    linhagem_disponivel = [coluna for coluna in COLUNAS_LINHAGEM if coluna in colunas]
    if linhagem_disponivel:
        nulos_linhagem = " OR ".join(
            f"{citar_identificador(coluna)} IS NULL" for coluna in linhagem_disponivel
        )
        expressoes.append(
            f"count(*) FILTER (WHERE {nulos_linhagem}) AS linhas_linhagem_incompleta"
        )
    else:
        expressoes.append("CAST(NULL AS BIGINT) AS linhas_linhagem_incompleta")

    if chave and all(coluna in colunas for coluna in chave):
        colunas_chave = ", ".join(citar_identificador(coluna) for coluna in chave)
        incompleta = " OR ".join(
            f"{citar_identificador(coluna)} IS NULL" for coluna in chave
        )
        completa = " AND ".join(
            f"{citar_identificador(coluna)} IS NOT NULL" for coluna in chave
        )
        expressoes.extend(
            [
                f"'{', '.join(chave)}' AS chave_candidata",
                f"count(*) FILTER (WHERE {incompleta}) AS linhas_chave_incompleta",
                "count(*) FILTER (WHERE "
                f"{completa}) - count(DISTINCT ({colunas_chave})) "
                f"FILTER (WHERE {completa}) AS linhas_chave_duplicada",
                "'candidata; medir, não deduplicar' AS status_chave",
            ]
        )
    else:
        expressoes.extend(
            [
                "CAST(NULL AS VARCHAR) AS chave_candidata",
                "CAST(NULL AS BIGINT) AS linhas_chave_incompleta",
                "CAST(NULL AS BIGINT) AS linhas_chave_duplicada",
                "'não validada; consulte a matriz Silver' AS status_chave",
            ]
        )

    for campo in ("linhas_bronze_preparadas", "linhas_saida", "linhas_rejeitadas", "copias_removidas"):
        valor = "CAST(NULL AS BIGINT)" if metricas is None else str(metricas[campo])
        expressoes.append(f"{valor} AS {campo}_manifesto")
    for campo in ("rejeicoes_por_motivo", "conversoes_opcionais_por_motivo"):
        valor = (
            "CAST(NULL AS VARCHAR)"
            if metricas is None
            else literal_sql(json.dumps(metricas[campo], ensure_ascii=False, sort_keys=True))
        )
        expressoes.append(f"{valor} AS {campo}_manifesto")
    expressoes.append(
        literal_sql("completo" if metricas is not None else "sem métricas em algum manifesto")
        + " AS status_metricas_manifesto"
    )

    return (
        f"CREATE OR REPLACE VIEW qualidade_{nome_view} AS\n"
        f"SELECT {', '.join(expressoes)}\nFROM {nome_view};"
    )


def main() -> int:
    catalogo: dict[str, dict[str, str]] = {}
    metricas_silver: dict[str, list[dict | None]] = {}
    tabelas_bronze = coletar(BRONZE, "bronze", catalogo)
    tabelas_silver = coletar(SILVER, "silver", catalogo, metricas_silver)
    tabelas = tabelas_bronze | tabelas_silver
    for nome_view, metadados in list(catalogo.items()):
        if metadados["camada"] != "silver" or nome_view not in tabelas_silver:
            continue
        nome_qualidade = f"qualidade_{nome_view}"
        catalogo[nome_qualidade] = {
            "camada": "qualidade",
            "fonte": metadados["fonte"],
            "recurso": metadados["recurso"],
        }
    if sys.argv[1:] == ["--catalogo-json"]:
        print(
            json.dumps(
                [
                    {
                        "nome": nome,
                        **{k: v for k, v in catalogo[nome].items() if k != "recurso"},
                    }
                    for nome in sorted(catalogo)
                ]
            )
        )
        return 0
    max_arquivos = int(os.environ.get("TODEOLHO_MAX_PARQUETS_POR_VIEW", "10000"))
    omitidas = []
    for nome in sorted(tabelas):
        arquivos = sorted(set(tabelas[nome]))
        if len(arquivos) > max_arquivos:
            omitidas.append((nome, len(arquivos)))
            continue
        lista = ", ".join("'" + a.replace("'", "''") + "'" for a in arquivos)
        print(f"CREATE OR REPLACE VIEW {nome} AS")
        print(
            f"  SELECT * FROM read_parquet([{lista}], "
            "union_by_name = true, hive_partitioning = true);"
        )
    for nome in sorted(tabelas_silver):
        metadados = catalogo[nome]
        recurso = metadados["recurso"]
        chave = chave_candidata(metadados["fonte"], recurso)
        colunas = inspecionar_colunas(sorted(set(tabelas_silver[nome])))
        if colunas:
            print(sql_qualidade(nome, colunas, chave, agregar_metricas(metricas_silver.get(nome, []))))
    for nome, quantidade in omitidas:
        print(f"-- OMITIDA: {nome} ({quantidade} arquivos > {max_arquivos})", file=sys.stderr)
    geradas = len(tabelas) - len(omitidas)
    print(f"-- {geradas} view(s) Bronze/Silver geradas", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
