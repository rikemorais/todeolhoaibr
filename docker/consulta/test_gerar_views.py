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
            manifesto_atual.write_text(
                json.dumps(
                    {
                        "fonte": "ana",
                        "recurso": "estacoes",
                        "partes": [{"chave": "ana/estacoes/data=2026/parte-1.parquet"}],
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

            tabelas = GERADOR.coletar(raiz, "silver")

            self.assertEqual(
                tabelas["silver_ana_estacoes"],
                [str(raiz / "ana/estacoes/data=2026/parte-1.parquet")],
            )


class TesteRelatorioQualidade(unittest.TestCase):
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
