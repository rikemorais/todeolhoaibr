"""Regressões da geração de views vigentes e perfis Silver."""

from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

ARQUIVO = Path(__file__).with_name("gerar_views.py")
ESPECIFICACAO = importlib.util.spec_from_file_location("gerar_views", ARQUIVO)
assert ESPECIFICACAO is not None and ESPECIFICACAO.loader is not None
GERADOR = importlib.util.module_from_spec(ESPECIFICACAO)
ESPECIFICACAO.loader.exec_module(GERADOR)


class TesteViewsVigentes(unittest.TestCase):
    def test_coleta_somente_partes_do_manifesto_apontado(self) -> None:
        with tempfile.TemporaryDirectory() as temporario:
            raiz = Path(temporario)
            recurso = raiz / "ana" / "estacoes"
            manifesto_atual = recurso / "data=2026" / "_manifesto.json"
            manifesto_antigo = recurso / "data=2025" / "_manifesto.json"
            manifesto_atual.parent.mkdir(parents=True)
            manifesto_antigo.parent.mkdir(parents=True)
            qualidade_atual = {"versao": 1, "linhas_saida": 7}
            manifesto_atual.write_text(
                json.dumps(
                    {
                        "fonte": "ana",
                        "recurso": "estacoes",
                        "partes": [{"chave": "ana/estacoes/data=2026/parte-1.parquet"}],
                        "qualidade": qualidade_atual,
                    }
                ),
                encoding="utf-8",
            )
            manifesto_antigo.write_text(
                json.dumps(
                    {
                        "fonte": "ana",
                        "recurso": "estacoes",
                        "partes": [{"chave": "ana/estacoes/data=2025/parte-1.parquet"}],
                    }
                ),
                encoding="utf-8",
            )
            (recurso / "_ultima_versao.json").write_text(
                json.dumps({"manifesto": "ana/estacoes/data=2026/_manifesto.json"}),
                encoding="utf-8",
            )

            metricas = {}
            tabelas = GERADOR.coletar(raiz, "silver", metricas_silver=metricas)

            self.assertEqual(
                tabelas["silver_ana_estacoes"],
                [str(raiz / "ana/estacoes/data=2026/parte-1.parquet")],
            )
            self.assertEqual(metricas["silver_ana_estacoes"], [qualidade_atual])


class TesteRelatorioQualidade(unittest.TestCase):
    def test_agrega_contagens_dos_manifestos_vigentes(self) -> None:
        metricas = GERADOR.agregar_metricas([
            {
                "versao": 2,
                "linhas_bronze_preparadas": 4,
                "linhas_saida": 3,
                "linhas_rejeitadas": 1,
                "copias_removidas": 1,
                "rejeicoes_por_motivo": {"invalido": 1},
                "conversoes_opcionais_por_motivo": {"data": 2},
            },
            {
                "versao": 2,
                "linhas_bronze_preparadas": 2,
                "linhas_saida": 2,
                "linhas_rejeitadas": 0,
                "copias_removidas": 0,
                "rejeicoes_por_motivo": {},
                "conversoes_opcionais_por_motivo": {"data": 1},
            },
        ])

        self.assertEqual(metricas["linhas_bronze_preparadas"], 6)
        self.assertEqual(metricas["linhas_saida"], 5)
        self.assertEqual(metricas["linhas_rejeitadas"], 1)
        self.assertEqual(metricas["copias_removidas"], 1)
        self.assertEqual(metricas["rejeicoes_por_motivo"], {"invalido": 1})
        self.assertEqual(metricas["conversoes_opcionais_por_motivo"], {"data": 3})
        sql = GERADOR.sql_qualidade("silver_ana_estacoes", ["codigo"], None, metricas)
        self.assertIn("6 AS linhas_bronze_preparadas_manifesto", sql)
        self.assertIn("5 AS linhas_saida_manifesto", sql)
        self.assertIn("1 AS linhas_rejeitadas_manifesto", sql)
        self.assertIn("1 AS copias_removidas_manifesto", sql)

    def test_nao_exibe_total_parcial_com_manifesto_antigo(self) -> None:
        self.assertIsNone(GERADOR.agregar_metricas([{"versao": 1}]))
        self.assertIsNone(GERADOR.agregar_metricas([
            {
                "versao": 2,
                "linhas_bronze_preparadas": 4,
                "linhas_saida": 4,
                "linhas_rejeitadas": 0,
                "copias_removidas": 0,
                "rejeicoes_por_motivo": {},
                "conversoes_opcionais_por_motivo": {},
            },
            None,
        ]))
        sql = GERADOR.sql_qualidade("silver_ana_estacoes", ["codigo"], None, None)
        self.assertIn("CAST(NULL AS BIGINT) AS linhas_saida_manifesto", sql)
        self.assertIn("sem métricas em algum manifesto", sql)

    def test_chave_mensal_de_uf_do_mapa_de_empresas(self) -> None:
        self.assertEqual(
            GERADOR.chave_candidata("mapa_empresas", "movimentacao"),
            ("periodo", "uf"),
        )

    def test_mede_nulos_linhagem_e_chave_candidata_sem_apagar_linhas(self) -> None:
        sql = GERADOR.sql_qualidade(
            "silver_ana_estacoes",
            ["codigo", "nome", "_tdoa_origem_manifesto", "_tdoa_origem_parte", "_tdoa_origem_ordinal"],
            ("codigo",),
        )

        self.assertIn("CREATE OR REPLACE VIEW qualidade_silver_ana_estacoes", sql)
        self.assertIn('AS "nulos__nome"', sql)
        self.assertIn("linhas_linhagem_incompleta", sql)
        self.assertIn("linhas_chave_incompleta", sql)
        self.assertIn("linhas_chave_duplicada", sql)
        self.assertIn("candidata; medir, não deduplicar", sql)
        self.assertNotIn("DISTINCT *", sql)

    def test_nao_inventa_medida_de_chave_quando_nao_foi_validada(self) -> None:
        sql = GERADOR.sql_qualidade("silver_remuneracao", ["valor"], None)

        self.assertIn("CAST(NULL AS BIGINT) AS linhas_chave_duplicada", sql)
        self.assertIn("não validada; consulte a matriz Silver", sql)

    def test_escapa_identificador_externo(self) -> None:
        self.assertEqual(GERADOR.citar_identificador('campo"externo'), '"campo""externo"')


if __name__ == "__main__":
    unittest.main()
