//! Adaptador de tarefa Dagron para contratos Silver SQL/DataFusion.

use std::sync::Arc;

use arrow::datatypes::SchemaRef;
use async_trait::async_trait;
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa, estado_na_raw,
};

use crate::{SnapshotBronze, executor_sql::ConfiguracaoExecutorSql};

use super::publicacao::{ConfiguracaoDePublicacao, publicar_transformacao_sql};

/// Tarefa reutilizável para executar um contrato SQL Silver sobre um snapshot Bronze.
pub struct TarefaDeSilverSql {
    bronze: Arc<dyn ArmazenamentoRaw>,
    snapshot: Arc<SnapshotBronze>,
    fonte: String,
    recurso: String,
    recurso_destino: Option<String>,
    regra: String,
    sql: &'static str,
    esquema: SchemaRef,
    executor: ConfiguracaoExecutorSql,
}

impl TarefaDeSilverSql {
    /// Cria uma tarefa SQL com configuração operacional padrão.
    #[allow(clippy::too_many_arguments)]
    pub fn nova(
        bronze: Arc<dyn ArmazenamentoRaw>,
        snapshot: Arc<SnapshotBronze>,
        fonte: impl Into<String>,
        recurso: impl Into<String>,
        recurso_destino: Option<String>,
        regra: impl Into<String>,
        sql: &'static str,
        esquema: SchemaRef,
    ) -> Self {
        Self {
            bronze,
            snapshot,
            fonte: fonte.into(),
            recurso: recurso.into(),
            recurso_destino,
            regra: regra.into(),
            sql,
            esquema,
            executor: ConfiguracaoExecutorSql::default(),
        }
    }

    /// Substitui os limites operacionais desta tarefa.
    pub fn com_executor(mut self, executor: ConfiguracaoExecutorSql) -> Self {
        self.executor = executor;
        self
    }

    /// Indica se a tabela publicada ainda precisa acompanhar o snapshot e a regra.
    pub async fn pendente(&self, silver: &dyn ArmazenamentoRaw) -> Result<bool, ErroDeIngestao> {
        let destino = self.recurso_destino.as_deref().unwrap_or(&self.recurso);
        let versao = format!(
            "{}|regra={}",
            self.snapshot.manifesto().id_execucao,
            self.regra
        );
        Ok(!matches!(
            estado_na_raw(silver, &self.fonte, destino).await?,
            Some(ref estado)
                if estado.completa && estado.integra && estado.versao.as_deref() == Some(versao.as_str())
        ))
    }
}

#[async_trait]
impl Tarefa for TarefaDeSilverSql {
    fn nome(&self) -> &str {
        self.recurso_destino.as_deref().unwrap_or(&self.recurso)
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        publicar_transformacao_sql(
            self.bronze.as_ref(),
            contexto.armazenamento.as_ref(),
            &self.snapshot,
            ConfiguracaoDePublicacao {
                fonte: &self.fonte,
                recurso: &self.recurso,
                recurso_destino: self.recurso_destino.as_deref(),
                regra: &self.regra,
                esquema: Some(self.esquema.clone()),
                iniciada_em: contexto.iniciada_em,
                id_execucao: &contexto.id_execucao,
            },
            self.sql,
            &self.executor,
        )
        .await
    }
}

#[cfg(test)]
mod testes {
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use arrow::array::{
        Array, Date32Array, Decimal128Array, Int64Array, StringArray, UInt16Array, UInt32Array,
        UInt64Array,
    };
    use arrow::datatypes::{DataType, Field, Schema};
    use async_trait::async_trait;
    use bytes::Bytes;
    use chrono::{TimeZone, Utc};
    use futures::StreamExt;
    use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
    use sha2::{Digest, Sha256};
    use todeolho_nucleo::{
        ClienteHttp, ContextoDeExecucao, DownloadHttp, ErroDeIngestao, InformacoesDoArquivo,
        LocalizacaoRaw, ManifestoRaw, ObjetoRaw, ParteRaw, RespostaHttp, Tarefa, UltimaVersao,
    };
    use tokio::io::{AsyncRead, AsyncReadExt};
    use tokio::sync::Notify;

    use super::*;
    use crate::{
        SnapshotBronze, Tabela,
        executor_sql::{executar_sql, preparar_snapshot as preparar_entrada_sql},
        silver::tarefa_tse_despesas_contratadas::publicar_tse_despesas_contratadas_2024,
        silver::tarefa_tse_despesas_contratadas_2018::publicar_tse_despesas_contratadas_2018,
        silver::tarefa_tse_despesas_contratadas_2020::publicar_tse_despesas_contratadas_2020,
        silver::tarefa_tse_despesas_contratadas_2022::publicar_tse_despesas_contratadas_2022,
        silver::tarefa_tse_despesas_pagas_2018::publicar_tse_despesas_pagas_2018,
        silver::tarefa_tse_despesas_pagas_2020::publicar_tse_despesas_pagas_2020,
        silver::tarefa_tse_despesas_pagas_2022::publicar_tse_despesas_pagas_2022,
        silver::tarefa_tse_receitas_candidatos::publicar_tse_receitas_candidatos_2024,
        silver::tarefa_tse_receitas_candidatos_2018::publicar_tse_receitas_candidatos_2018,
        silver::tarefa_tse_receitas_candidatos_2020::publicar_tse_receitas_candidatos_2020,
        silver::tarefa_tse_receitas_candidatos_2022::publicar_tse_receitas_candidatos_2022,
        silver::tarefa_tse_receitas_doador_originario::publicar_tse_receitas_doador_originario_2024,
        silver::tarefa_tse_receitas_doador_originario_2018::publicar_tse_receitas_doador_originario_2018,
        silver::tarefa_tse_receitas_doador_originario_2020::publicar_tse_receitas_doador_originario_2020,
        silver::tarefa_tse_receitas_doador_originario_2022::publicar_tse_receitas_doador_originario_2022,
    };

    const FONTE: &str = "teste-sql";
    const RECURSO: &str = "recurso";
    const REGRA: &str = "teste-v1";
    const SQL: &str = "SELECT CAST(a.id AS BIGINT) AS id, CAST(a.valor AS BIGINT) AS valor, a._tdoa_origem_manifesto AS _tdoa_origem_manifesto, a._tdoa_origem_parte AS _tdoa_origem_parte, a._tdoa_origem_ordinal AS _tdoa_origem_ordinal FROM bronze AS a INNER JOIN bronze AS b ON a.id = b.id WHERE CAST(a.valor AS BIGINT) >= 20";

    #[derive(Default)]
    struct Memoria {
        objetos: Mutex<HashMap<String, Vec<u8>>>,
        leituras_de_partes: AtomicUsize,
        bloqueio_upload: Mutex<Option<Arc<ControleUpload>>>,
    }

    #[derive(Default)]
    struct ControleUpload {
        iniciou: Notify,
        continuar: Notify,
    }

    #[async_trait]
    impl ArmazenamentoRaw for Memoria {
        async fn preparar(&self) -> Result<(), ErroDeIngestao> {
            Ok(())
        }

        async fn gravar(&self, objeto: ObjetoRaw) -> Result<(), ErroDeIngestao> {
            self.objetos
                .lock()
                .unwrap()
                .insert(objeto.chave, objeto.conteudo);
            Ok(())
        }

        async fn gravar_fluxo(
            &self,
            chave: &str,
            _: &str,
            fluxo: &mut (dyn AsyncRead + Send + Unpin),
        ) -> Result<(), ErroDeIngestao> {
            let mut conteudo = Vec::new();
            fluxo.read_to_end(&mut conteudo).await.map_err(|erro| {
                ErroDeIngestao::Armazenamento {
                    chave: chave.to_owned(),
                    detalhe: erro.to_string(),
                }
            })?;
            let bloqueio = self.bloqueio_upload.lock().unwrap().clone();
            if let Some(bloqueio) = bloqueio {
                bloqueio.iniciou.notify_one();
                bloqueio.continuar.notified().await;
            }
            self.objetos
                .lock()
                .unwrap()
                .insert(chave.to_owned(), conteudo);
            Ok(())
        }

        async fn ler(&self, chave: &str) -> Result<Option<Vec<u8>>, ErroDeIngestao> {
            Ok(self.objetos.lock().unwrap().get(chave).cloned())
        }

        async fn ler_fluxo(
            &self,
            chave: &str,
            escrita: &mut (dyn tokio::io::AsyncWrite + Send + Unpin),
        ) -> Result<bool, ErroDeIngestao> {
            let Some(conteudo) = self.ler(chave).await? else {
                return Ok(false);
            };
            self.leituras_de_partes.fetch_add(1, Ordering::Relaxed);
            tokio::io::AsyncWriteExt::write_all(escrita, &conteudo)
                .await
                .map_err(|erro| ErroDeIngestao::Armazenamento {
                    chave: chave.to_owned(),
                    detalhe: erro.to_string(),
                })?;
            Ok(true)
        }
    }

    struct HttpInerte;

    #[async_trait]
    impl ClienteHttp for HttpInerte {
        async fn obter(&self, _: &str, _: &str) -> Result<RespostaHttp, ErroDeIngestao> {
            unreachable!("a tarefa Silver SQL não acessa HTTP")
        }

        async fn informacoes(&self, _: &str) -> Result<InformacoesDoArquivo, ErroDeIngestao> {
            unreachable!("a tarefa Silver SQL não acessa HTTP")
        }

        async fn baixar(&self, _: &str) -> Result<DownloadHttp, ErroDeIngestao> {
            unreachable!("a tarefa Silver SQL não acessa HTTP")
        }
    }

    fn parquet(linhas: &[(&str, &str)]) -> Vec<u8> {
        let mut tabela = Tabela::nova(["id", "valor"]);
        for (id, valor) in linhas {
            tabela = tabela.com_linha([Some((*id).into()), Some((*valor).into())]);
        }
        tabela.para_parquet().unwrap()
    }

    async fn preparar_snapshot(bronze: &Memoria) -> SnapshotBronze {
        let instante = Utc.with_ymd_and_hms(2026, 11, 6, 12, 0, 0).unwrap();
        let local = LocalizacaoRaw::nova(FONTE, RECURSO, instante, "bronze-1");
        let mut partes = Vec::new();
        for (indice, linhas) in [
            vec![("1", "10"), ("2", "20")],
            vec![("3", "30"), ("4", "40")],
        ]
        .into_iter()
        .enumerate()
        {
            let conteudo = parquet(&linhas);
            let parte = ParteRaw {
                chave: local.parte(indice + 1, "parquet"),
                url: format!("parte-{indice}"),
                bytes: conteudo.len() as u64,
                sha256: hex::encode(Sha256::digest(&conteudo)),
                tipo_de_conteudo: "application/vnd.apache.parquet".into(),
                versao: Some("bronze-1".into()),
            };
            bronze
                .gravar(ObjetoRaw {
                    chave: parte.chave.clone(),
                    conteudo,
                    tipo_de_conteudo: "application/vnd.apache.parquet".into(),
                })
                .await
                .unwrap();
            partes.push(parte);
        }
        let obsoleta = parquet(&[("999", "999")]);
        bronze
            .gravar(ObjetoRaw {
                chave: "teste-sql/recurso/execucao-antiga/parte.parquet".into(),
                conteudo: obsoleta,
                tipo_de_conteudo: "application/vnd.apache.parquet".into(),
            })
            .await
            .unwrap();
        let manifesto = ManifestoRaw {
            fonte: FONTE.into(),
            recurso: RECURSO.into(),
            id_execucao: "bronze-1".into(),
            url_inicial: "fixture".into(),
            iniciada_em: instante,
            concluida_em: instante,
            partes,
        };
        let chave_manifesto = local.manifesto();
        bronze
            .gravar(ObjetoRaw {
                chave: chave_manifesto.clone(),
                conteudo: serde_json::to_vec(&manifesto).unwrap(),
                tipo_de_conteudo: "application/json".into(),
            })
            .await
            .unwrap();
        bronze
            .gravar(ObjetoRaw {
                chave: LocalizacaoRaw::ultima_versao(FONTE, RECURSO),
                conteudo: serde_json::to_vec(&UltimaVersao {
                    versao: Some("bronze-1".into()),
                    manifesto: chave_manifesto,
                    iniciada_em: Some(instante),
                })
                .unwrap(),
                tipo_de_conteudo: "application/json".into(),
            })
            .await
            .unwrap();
        SnapshotBronze::vigente(bronze, FONTE, RECURSO)
            .await
            .unwrap()
            .unwrap()
    }

    async fn preparar_snapshot_receitas(bronze: &Memoria, ano: u16) -> SnapshotBronze {
        let instante = Utc.with_ymd_and_hms(2026, 11, 10, 12, 0, 0).unwrap();
        let ano_texto = ano.to_string();
        let recurso = format!("prestacao-contas/ano={ano}");
        let id_bronze = format!("bronze-receitas-{ano}");
        let mut tabela = Tabela::nova([
            "AA_ELEICAO",
            "CD_ELEICAO",
            "SQ_PRESTADOR_CONTAS",
            "SQ_RECEITA",
            "DT_RECEITA",
            "VR_RECEITA",
            "DS_RECEITA",
        ])
        .com_linha([
            Some(ano_texto.clone()),
            Some("547".into()),
            Some("123456789012345".into()),
            Some("-1".into()),
            Some(format!("15/11/{ano}")),
            Some("123,45".into()),
            Some("recurso sem sequência".into()),
        ])
        .com_linha([
            Some(ano_texto.clone()),
            Some("547".into()),
            Some("123456789012345".into()),
            Some("500".into()),
            Some(format!("16/11/{ano}")),
            Some("250,00".into()),
            Some("doação estimável".into()),
        ])
        .com_linha([
            Some(ano_texto.clone()),
            Some("547".into()),
            Some("123456789012345".into()),
            Some("500".into()),
            Some(format!("16/11/{ano}")),
            Some("275,50".into()),
            Some("doação estimável retificada".into()),
        ])
        .com_linha([
            Some(ano_texto.clone()),
            Some("547".into()),
            Some("123456789012345".into()),
            Some("-1".into()),
            Some("-1".into()),
            Some("15,50".into()),
            Some("data não informada".into()),
        ]);
        if ano == 2018 {
            tabela = tabela.com_linha([
                Some(ano_texto.clone()),
                Some("547".into()),
                Some("123456789012345".into()),
                Some("500".into()),
                Some(format!("16/11/{ano}")),
                Some("250,00".into()),
                Some("doação estimável".into()),
            ]);
        }
        if ano == 2020 {
            tabela = tabela
                .com_linha([
                    Some(ano_texto.clone()),
                    Some("547".into()),
                    Some("123456789012345".into()),
                    Some("-3".into()),
                    Some(String::new()),
                    Some("20,00".into()),
                    Some("sequência sentinela".into()),
                ])
                .com_linha([
                    Some(ano_texto.clone()),
                    Some("547".into()),
                    Some("123456789012345".into()),
                    Some(String::new()),
                    Some(String::new()),
                    Some("30,00".into()),
                    Some("sequência ausente".into()),
                ]);
        }
        let conteudo = tabela.para_parquet().unwrap();
        let local = LocalizacaoRaw::nova("tse", &recurso, instante, &id_bronze);
        let parte = ParteRaw {
            chave: local.parte(1, "parquet"),
            url: format!("https://dados.tse.jus.br/receitas_candidatos_{ano}_BRASIL.csv"),
            bytes: conteudo.len() as u64,
            sha256: hex::encode(Sha256::digest(&conteudo)),
            tipo_de_conteudo: "application/vnd.apache.parquet".into(),
            versao: Some(id_bronze.clone()),
        };
        bronze
            .gravar(ObjetoRaw {
                chave: parte.chave.clone(),
                conteudo,
                tipo_de_conteudo: "application/vnd.apache.parquet".into(),
            })
            .await
            .unwrap();
        let manifesto = ManifestoRaw {
            fonte: "tse".into(),
            recurso: recurso.clone(),
            id_execucao: id_bronze.clone(),
            url_inicial: "fixture".into(),
            iniciada_em: instante,
            concluida_em: instante,
            partes: vec![parte],
        };
        let chave_manifesto = local.manifesto();
        bronze
            .gravar(ObjetoRaw {
                chave: chave_manifesto.clone(),
                conteudo: serde_json::to_vec(&manifesto).unwrap(),
                tipo_de_conteudo: "application/json".into(),
            })
            .await
            .unwrap();
        bronze
            .gravar(ObjetoRaw {
                chave: LocalizacaoRaw::ultima_versao("tse", &recurso),
                conteudo: serde_json::to_vec(&UltimaVersao {
                    versao: Some(id_bronze.clone()),
                    manifesto: chave_manifesto,
                    iniciada_em: Some(instante),
                })
                .unwrap(),
                tipo_de_conteudo: "application/json".into(),
            })
            .await
            .unwrap();
        SnapshotBronze::vigente(bronze, "tse", &recurso)
            .await
            .unwrap()
            .unwrap()
    }

    async fn preparar_snapshot_despesas_contratadas(bronze: &Memoria, ano: u16) -> SnapshotBronze {
        let instante = Utc.with_ymd_and_hms(2026, 11, 10, 12, 0, 0).unwrap();
        let ano_texto = ano.to_string();
        let recurso = format!("prestacao-contas/ano={ano}");
        let id_bronze = format!("bronze-despesas-contratadas-{ano}");
        let colunas = [
            "AA_ELEICAO",
            "CD_ELEICAO",
            "SQ_PRESTADOR_CONTAS",
            "SQ_DESPESA",
            "DT_DESPESA",
            "DS_DESPESA",
            "VR_DESPESA_CONTRATADA",
        ];
        let nacional = Tabela::nova(colunas)
            .com_linha([
                Some(ano_texto.clone()),
                Some("547".into()),
                Some("123456789012345".into()),
                Some("7".into()),
                Some(format!("10/10/{ano}")),
                Some("serviço contratado".into()),
                Some("100,00".into()),
            ])
            .com_linha([
                Some(ano_texto.clone()),
                Some("547".into()),
                Some("123456789012345".into()),
                Some("7".into()),
                Some(format!("10/10/{ano}")),
                Some("serviço contratado retificado".into()),
                Some("120,00".into()),
            ])
            .com_linha([
                Some(ano_texto.clone()),
                Some("547".into()),
                Some("123456789012345".into()),
                Some("-1".into()),
                Some("-1".into()),
                Some("despesa sem sequência".into()),
                Some("50,25".into()),
            ])
            .com_linha([
                Some(ano_texto.clone()),
                Some("547".into()),
                Some("123456789012345".into()),
                Some("-3".into()),
                Some("#NE".into()),
                Some("".into()),
                Some("25,00".into()),
            ]);
        let nacional = if ano == 2020 || ano == 2018 {
            nacional.com_linha([
                Some(ano_texto.clone()),
                Some("547".into()),
                Some("123456789012345".into()),
                Some("7".into()),
                Some(format!("10/10/{ano}")),
                Some("serviço contratado".into()),
                Some("100,00".into()),
            ])
        } else {
            nacional
        };
        let estadual = Tabela::nova(colunas).com_linha([
            Some(ano_texto.clone()),
            Some("547".into()),
            Some("123456789012345".into()),
            Some("8".into()),
            Some(format!("11/10/{ano}")),
            Some("parte estadual".into()),
            Some("10,00".into()),
        ]);
        let local = LocalizacaoRaw::nova("tse", &recurso, instante, &id_bronze);
        let mut partes = Vec::new();
        for (indice, (nome, tabela)) in [
            (
                format!("despesas_contratadas_candidatos_{ano}_BRASIL.csv"),
                nacional,
            ),
            (
                format!("despesas_contratadas_candidatos_{ano}_AC.csv"),
                estadual,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let conteudo = tabela.para_parquet().unwrap();
            let parte = ParteRaw {
                chave: local.parte(indice + 1, "parquet"),
                url: format!("https://dados.tse.jus.br/{nome}"),
                bytes: conteudo.len() as u64,
                sha256: hex::encode(Sha256::digest(&conteudo)),
                tipo_de_conteudo: "application/vnd.apache.parquet".into(),
                versao: Some(id_bronze.clone()),
            };
            bronze
                .gravar(ObjetoRaw {
                    chave: parte.chave.clone(),
                    conteudo,
                    tipo_de_conteudo: "application/vnd.apache.parquet".into(),
                })
                .await
                .unwrap();
            partes.push(parte);
        }
        let manifesto = ManifestoRaw {
            fonte: "tse".into(),
            recurso: recurso.clone(),
            id_execucao: id_bronze.clone(),
            url_inicial: "fixture".into(),
            iniciada_em: instante,
            concluida_em: instante,
            partes,
        };
        let chave_manifesto = local.manifesto();
        bronze
            .gravar(ObjetoRaw {
                chave: chave_manifesto.clone(),
                conteudo: serde_json::to_vec(&manifesto).unwrap(),
                tipo_de_conteudo: "application/json".into(),
            })
            .await
            .unwrap();
        bronze
            .gravar(ObjetoRaw {
                chave: LocalizacaoRaw::ultima_versao("tse", &recurso),
                conteudo: serde_json::to_vec(&UltimaVersao {
                    versao: Some(id_bronze.clone()),
                    manifesto: chave_manifesto,
                    iniciada_em: Some(instante),
                })
                .unwrap(),
                tipo_de_conteudo: "application/json".into(),
            })
            .await
            .unwrap();
        SnapshotBronze::vigente(bronze, "tse", &recurso)
            .await
            .unwrap()
            .unwrap()
    }

    async fn preparar_snapshot_despesas_pagas(bronze: &Memoria, ano: u16) -> SnapshotBronze {
        let instante = Utc.with_ymd_and_hms(2026, 11, 10, 12, 0, 0).unwrap();
        let ano_texto = ano.to_string();
        let recurso = format!("prestacao-contas/ano={ano}");
        let id_bronze = format!("bronze-despesas-pagas-{ano}");
        let colunas = [
            "AA_ELEICAO",
            "CD_ELEICAO",
            "SQ_PRESTADOR_CONTAS",
            "SQ_DESPESA",
            "SQ_PARCELAMENTO_DESPESA",
            "DT_PAGTO_DESPESA",
            "VR_PAGTO_DESPESA",
        ];
        let linha = |pagamento: &str, valor: &str| {
            [
                Some(ano_texto.clone()),
                Some("547".into()),
                Some("123456789012345".into()),
                Some("200".into()),
                Some(pagamento.into()),
                Some(format!("16/08/{ano}")),
                Some(valor.into()),
            ]
        };
        let nacional = Tabela::nova(colunas)
            .com_linha(linha("1", "125,50"))
            .com_linha(linha("1", "125,50"))
            .com_linha(linha("2", "50,00"));
        let estadual = Tabela::nova(colunas).com_linha([
            Some(ano_texto.clone()),
            Some("547".into()),
            Some("123456789012345".into()),
            Some("201".into()),
            Some("1".into()),
            Some(format!("17/08/{ano}")),
            Some("10,00".into()),
        ]);
        let local = LocalizacaoRaw::nova("tse", &recurso, instante, &id_bronze);
        let mut partes = Vec::new();
        for (indice, (nome, tabela)) in [
            (
                format!("despesas_pagas_candidatos_{ano}_BRASIL.csv"),
                nacional,
            ),
            (format!("despesas_pagas_candidatos_{ano}_AC.csv"), estadual),
        ]
        .into_iter()
        .enumerate()
        {
            let conteudo = tabela.para_parquet().unwrap();
            let parte = ParteRaw {
                chave: local.parte(indice + 1, "parquet"),
                url: format!("https://dados.tse.jus.br/{nome}"),
                bytes: conteudo.len() as u64,
                sha256: hex::encode(Sha256::digest(&conteudo)),
                tipo_de_conteudo: "application/vnd.apache.parquet".into(),
                versao: Some(id_bronze.clone()),
            };
            bronze
                .gravar(ObjetoRaw {
                    chave: parte.chave.clone(),
                    conteudo,
                    tipo_de_conteudo: "application/vnd.apache.parquet".into(),
                })
                .await
                .unwrap();
            partes.push(parte);
        }
        let manifesto = ManifestoRaw {
            fonte: "tse".into(),
            recurso: recurso.clone(),
            id_execucao: id_bronze.clone(),
            url_inicial: "fixture".into(),
            iniciada_em: instante,
            concluida_em: instante,
            partes,
        };
        let chave_manifesto = local.manifesto();
        bronze
            .gravar(ObjetoRaw {
                chave: chave_manifesto.clone(),
                conteudo: serde_json::to_vec(&manifesto).unwrap(),
                tipo_de_conteudo: "application/json".into(),
            })
            .await
            .unwrap();
        bronze
            .gravar(ObjetoRaw {
                chave: LocalizacaoRaw::ultima_versao("tse", &recurso),
                conteudo: serde_json::to_vec(&UltimaVersao {
                    versao: Some(id_bronze.clone()),
                    manifesto: chave_manifesto,
                    iniciada_em: Some(instante),
                })
                .unwrap(),
                tipo_de_conteudo: "application/json".into(),
            })
            .await
            .unwrap();
        SnapshotBronze::vigente(bronze, "tse", &recurso)
            .await
            .unwrap()
            .unwrap()
    }

    async fn preparar_snapshot_receitas_doador_originario(
        bronze: &Memoria,
        ano: u16,
    ) -> SnapshotBronze {
        let instante = Utc.with_ymd_and_hms(2026, 11, 10, 12, 0, 0).unwrap();
        let ano_texto = ano.to_string();
        let recurso = format!("prestacao-contas/ano={ano}");
        let id_bronze = format!("bronze-receitas-doador-originario-{ano}");
        let colunas = [
            "AA_ELEICAO",
            "CD_ELEICAO",
            "SQ_PRESTADOR_CONTAS",
            "NR_CPF_CNPJ_DOADOR_ORIGINARIO",
            "NM_DOADOR_ORIGINARIO",
            "NM_DOADOR_ORIGINARIO_RFB",
            "TP_DOADOR_ORIGINARIO",
            "CD_CNAE_DOADOR_ORIGINARIO",
            "DS_CNAE_DOADOR_ORIGINARIO",
            "SQ_RECEITA",
            "DT_RECEITA",
            "DS_RECEITA",
            "VR_RECEITA",
        ];
        let mut nacional = Tabela::nova(colunas)
            .com_linha([
                Some(ano_texto.clone()),
                Some("547".into()),
                Some("123456789012345".into()),
                Some("00123456789".into()),
                Some("Doadora de teste".into()),
                Some("Doadora de teste RFB".into()),
                Some("FISICA".into()),
                Some("1234567".into()),
                Some("Atividade de teste".into()),
                Some("700".into()),
                Some(format!("10/10/{ano}")),
                Some("doação declarada".into()),
                Some("100,00".into()),
            ])
            .com_linha([
                Some(ano_texto.clone()),
                Some("547".into()),
                Some("123456789012345".into()),
                Some("00123456789".into()),
                Some("Doadora de teste retificado".into()),
                Some("Doadora de teste RFB".into()),
                Some("FISICA".into()),
                Some("1234567".into()),
                Some("Atividade de teste".into()),
                Some("700".into()),
                Some(format!("10/10/{ano}")),
                Some("doação retificada".into()),
                Some("125,50".into()),
            ])
            .com_linha([
                Some(ano_texto.clone()),
                Some("547".into()),
                Some("123456789012345".into()),
                Some("-1".into()),
                Some("Doador sem identificação".into()),
                Some("#NULO".into()),
                Some("#NE".into()),
                Some("#NULO".into()),
                Some("#NE".into()),
                Some("-1".into()),
                Some("-1".into()),
                Some("".into()),
                Some("25,00".into()),
            ]);
        if ano == 2020 || ano == 2018 {
            nacional = nacional.com_linha([
                Some(ano_texto.clone()),
                Some("547".into()),
                Some("123456789012345".into()),
                Some("00123456789".into()),
                Some("Doadora de teste".into()),
                Some("Doadora de teste RFB".into()),
                Some("FISICA".into()),
                Some("1234567".into()),
                Some("Atividade de teste".into()),
                Some("700".into()),
                Some(format!("10/10/{ano}")),
                Some("doação declarada".into()),
                Some("100,00".into()),
            ]);
        }
        let estadual = Tabela::nova(colunas).com_linha([
            Some(ano_texto),
            Some("547".into()),
            Some("123456789012345".into()),
            Some("00987654321".into()),
            Some("Doador estadual de teste".into()),
            Some("Doador estadual de teste RFB".into()),
            Some("FISICA".into()),
            Some("7654321".into()),
            Some("Outra atividade".into()),
            Some("701".into()),
            Some(format!("11/10/{ano}")),
            Some("parte estadual".into()),
            Some("10,00".into()),
        ]);
        let local = LocalizacaoRaw::nova("tse", &recurso, instante, &id_bronze);
        let mut partes = Vec::new();
        for (indice, (nome, tabela)) in [
            (
                format!("receitas_candidatos_doador_originario_{ano}_BRASIL.csv"),
                nacional,
            ),
            (
                format!("receitas_candidatos_doador_originario_{ano}_AC.csv"),
                estadual,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let conteudo = tabela.para_parquet().unwrap();
            let parte = ParteRaw {
                chave: local.parte(indice + 1, "parquet"),
                url: format!("https://dados.tse.jus.br/{nome}"),
                bytes: conteudo.len() as u64,
                sha256: hex::encode(Sha256::digest(&conteudo)),
                tipo_de_conteudo: "application/vnd.apache.parquet".into(),
                versao: Some(id_bronze.clone()),
            };
            bronze
                .gravar(ObjetoRaw {
                    chave: parte.chave.clone(),
                    conteudo,
                    tipo_de_conteudo: "application/vnd.apache.parquet".into(),
                })
                .await
                .unwrap();
            partes.push(parte);
        }
        let manifesto = ManifestoRaw {
            fonte: "tse".into(),
            recurso: recurso.clone(),
            id_execucao: id_bronze.clone(),
            url_inicial: "fixture".into(),
            iniciada_em: instante,
            concluida_em: instante,
            partes,
        };
        let chave_manifesto = local.manifesto();
        bronze
            .gravar(ObjetoRaw {
                chave: chave_manifesto.clone(),
                conteudo: serde_json::to_vec(&manifesto).unwrap(),
                tipo_de_conteudo: "application/json".into(),
            })
            .await
            .unwrap();
        bronze
            .gravar(ObjetoRaw {
                chave: LocalizacaoRaw::ultima_versao("tse", &recurso),
                conteudo: serde_json::to_vec(&UltimaVersao {
                    versao: Some(id_bronze),
                    manifesto: chave_manifesto,
                    iniciada_em: Some(instante),
                })
                .unwrap(),
                tipo_de_conteudo: "application/json".into(),
            })
            .await
            .unwrap();
        SnapshotBronze::vigente(bronze, "tse", &recurso)
            .await
            .unwrap()
            .unwrap()
    }

    fn esquema_saida() -> SchemaRef {
        Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, true),
            Field::new("valor", DataType::Int64, true),
            Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
            Field::new("_tdoa_origem_parte", DataType::Utf8, false),
            Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
        ]))
    }

    fn nova_tarefa(
        bronze: Arc<Memoria>,
        snapshot: Arc<SnapshotBronze>,
        regra: &str,
        sql: &'static str,
    ) -> TarefaDeSilverSql {
        TarefaDeSilverSql::nova(
            bronze,
            snapshot,
            FONTE,
            RECURSO,
            None,
            regra,
            sql,
            esquema_saida(),
        )
        .com_executor(ConfiguracaoExecutorSql {
            limite_entrada_bytes: 1_000_000,
            limite_memoria_bytes: 64 * 1024 * 1024,
            limite_spill_bytes: 64 * 1024 * 1024,
            particoes: 1,
            tamanho_lote: 1,
        })
    }

    fn contexto(silver: Arc<Memoria>, id_execucao: &str) -> ContextoDeExecucao {
        ContextoDeExecucao {
            id_execucao: id_execucao.into(),
            iniciada_em: Utc.with_ymd_and_hms(2026, 11, 6, 12, 30, 0).unwrap(),
            armazenamento: silver,
            http: Arc::new(HttpInerte),
            progresso: None,
        }
    }

    #[tokio::test]
    async fn tarefa_sql_filtra_snapshot_preserva_linhagem_publica_e_reaproveita() {
        let bronze = Arc::new(Memoria::default());
        let snapshot = Arc::new(preparar_snapshot(&bronze).await);
        let chaves_partes_bronze = snapshot
            .manifesto()
            .partes
            .iter()
            .map(|parte| parte.chave.clone())
            .collect::<Vec<_>>();
        let silver = Arc::new(Memoria::default());
        let tarefa = nova_tarefa(Arc::clone(&bronze), Arc::clone(&snapshot), REGRA, SQL);

        assert_eq!(tarefa.nome(), RECURSO);
        assert!(tarefa.pendente(silver.as_ref()).await.unwrap());
        let resumo = tarefa
            .executar(&contexto(Arc::clone(&silver), "silver-1"))
            .await
            .unwrap();
        assert_eq!(resumo.partes, 1);
        assert!(!tarefa.pendente(silver.as_ref()).await.unwrap());

        let bytes_ponteiro = silver
            .ler(&LocalizacaoRaw::ultima_versao(FONTE, RECURSO))
            .await
            .unwrap()
            .unwrap();
        let ponteiro: UltimaVersao = serde_json::from_slice(&bytes_ponteiro).unwrap();
        let bytes_manifesto = silver.ler(&ponteiro.manifesto).await.unwrap().unwrap();
        let manifesto: ManifestoRaw = serde_json::from_slice(&bytes_manifesto).unwrap();
        assert_eq!(manifesto.partes.len(), 1);
        let bytes_parquet = silver
            .ler(&manifesto.partes[0].chave)
            .await
            .unwrap()
            .unwrap();
        let leitor = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(bytes_parquet))
            .unwrap()
            .with_batch_size(1)
            .build()
            .unwrap();
        let mut ids = Vec::new();
        let mut origens = Vec::new();
        let mut ordinais = Vec::new();
        for lote in leitor {
            let lote = lote.unwrap();
            ids.extend(
                lote.column(0)
                    .as_any()
                    .downcast_ref::<Int64Array>()
                    .unwrap()
                    .values()
                    .iter()
                    .copied(),
            );
            origens.extend(
                lote.column(3)
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .unwrap()
                    .iter()
                    .map(|valor| valor.unwrap().to_owned()),
            );
            ordinais.extend(
                lote.column(4)
                    .as_any()
                    .downcast_ref::<UInt64Array>()
                    .unwrap()
                    .values()
                    .iter()
                    .copied(),
            );
        }
        assert_eq!(ids, [2, 3, 4]);
        assert_eq!(
            origens,
            [
                chaves_partes_bronze[0].clone(),
                chaves_partes_bronze[1].clone(),
                chaves_partes_bronze[1].clone()
            ]
        );
        assert_eq!(ordinais, [2, 1, 2]);

        let leituras_antes = bronze.leituras_de_partes.load(Ordering::Relaxed);
        let repetido = tarefa
            .executar(&contexto(Arc::clone(&silver), "silver-2"))
            .await
            .unwrap();
        assert!(repetido.reaproveitada);
        assert_eq!(
            bronze.leituras_de_partes.load(Ordering::Relaxed),
            leituras_antes
        );
    }

    #[tokio::test]
    async fn despesas_contratadas_tse_preserva_variantes_e_sentinelas() {
        let bronze = Arc::new(Memoria::default());
        let snapshot = preparar_snapshot_despesas_contratadas(&bronze, 2024).await;
        let silver = Arc::new(Memoria::default());
        let resumo = publicar_tse_despesas_contratadas_2024(
            bronze.as_ref(),
            silver.as_ref(),
            &snapshot,
            Utc.with_ymd_and_hms(2026, 11, 10, 12, 30, 0).unwrap(),
            "silver-despesas-contratadas",
        )
        .await
        .unwrap();
        assert_eq!(resumo.partes, 1);

        let ponteiro = silver
            .ler(&LocalizacaoRaw::ultima_versao(
                "tse",
                "prestacao-contas/ano=2024/despesas-contratadas",
            ))
            .await
            .unwrap()
            .unwrap();
        let ponteiro: UltimaVersao = serde_json::from_slice(&ponteiro).unwrap();
        let manifesto: ManifestoRaw =
            serde_json::from_slice(&silver.ler(&ponteiro.manifesto).await.unwrap().unwrap())
                .unwrap();
        let parquet = silver
            .ler(&manifesto.partes[0].chave)
            .await
            .unwrap()
            .unwrap();
        let leitor = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(parquet))
            .unwrap()
            .with_batch_size(10)
            .build()
            .unwrap();
        let lotes = leitor.map(Result::unwrap).collect::<Vec<_>>();
        assert_eq!(lotes.iter().map(|lote| lote.num_rows()).sum::<usize>(), 4);
        let lote = &lotes[0];

        let sequencias = lote
            .column(3)
            .as_any()
            .downcast_ref::<UInt32Array>()
            .unwrap();
        assert_eq!(sequencias.null_count(), 2);
        assert_eq!(sequencias.value(0), 7);
        assert_eq!(sequencias.value(1), 7);

        let datas = lote
            .column(4)
            .as_any()
            .downcast_ref::<Date32Array>()
            .unwrap();
        assert_eq!(datas.null_count(), 2);
        let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
        let dia_10 = chrono::NaiveDate::from_ymd_opt(2024, 10, 10)
            .unwrap()
            .signed_duration_since(epoch)
            .num_days() as i32;
        assert_eq!(datas.value(0), dia_10);
        assert_eq!(datas.value(1), dia_10);

        let descricoes = lote
            .column(5)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert_eq!(descricoes.null_count(), 1);
        assert_eq!(descricoes.value(0), "serviço contratado");
        assert_eq!(descricoes.value(1), "serviço contratado retificado");

        let valores = lote
            .column(6)
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .unwrap();
        let mut valores_centavos = (0..valores.len())
            .map(|indice| valores.value(indice))
            .collect::<Vec<_>>();
        valores_centavos.sort_unstable();
        assert_eq!(valores_centavos, [2_500, 5_025, 10_000, 12_000]);

        let ordinais = lote
            .column(10)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap();
        let mut ordinais_publicados = ordinais.values().to_vec();
        ordinais_publicados.sort_unstable();
        assert_eq!(ordinais_publicados, [1, 2, 3, 4]);

        let originais = lote
            .column(7)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert!(
            (0..originais.len())
                .any(|i| originais.value(i).contains("serviço contratado retificado"))
        );
    }

    #[tokio::test]
    async fn despesas_contratadas_tse_2018_preserva_copias_exatas_e_sentinelas() {
        let bronze = Arc::new(Memoria::default());
        let snapshot = preparar_snapshot_despesas_contratadas(&bronze, 2018).await;
        let silver = Arc::new(Memoria::default());
        let resumo = publicar_tse_despesas_contratadas_2018(
            bronze.as_ref(),
            silver.as_ref(),
            &snapshot,
            Utc.with_ymd_and_hms(2026, 11, 10, 12, 30, 0).unwrap(),
            "silver-despesas-contratadas-2018",
        )
        .await
        .unwrap();
        assert_eq!(resumo.partes, 1);

        let bytes_ponteiro = silver
            .ler(&LocalizacaoRaw::ultima_versao(
                "tse",
                "prestacao-contas/ano=2018/despesas-contratadas",
            ))
            .await
            .unwrap()
            .unwrap();
        let ponteiro: UltimaVersao = serde_json::from_slice(&bytes_ponteiro).unwrap();
        let manifesto: ManifestoRaw =
            serde_json::from_slice(&silver.ler(&ponteiro.manifesto).await.unwrap().unwrap())
                .unwrap();
        let parquet = silver
            .ler(&manifesto.partes[0].chave)
            .await
            .unwrap()
            .unwrap();
        let leitor = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(parquet))
            .unwrap()
            .with_batch_size(10)
            .build()
            .unwrap();
        let lotes = leitor.map(Result::unwrap).collect::<Vec<_>>();
        assert_eq!(lotes.iter().map(|lote| lote.num_rows()).sum::<usize>(), 5);
        let lote = &lotes[0];

        let anos = lote
            .column(0)
            .as_any()
            .downcast_ref::<UInt16Array>()
            .unwrap();
        assert!(anos.values().iter().all(|ano| *ano == 2018));
        let sequencias = lote
            .column(3)
            .as_any()
            .downcast_ref::<UInt32Array>()
            .unwrap();
        assert_eq!(sequencias.null_count(), 2);
        assert_eq!(
            (0..sequencias.len())
                .filter(|indice| !sequencias.is_null(*indice) && sequencias.value(*indice) == 7)
                .count(),
            3
        );
        let datas = lote
            .column(4)
            .as_any()
            .downcast_ref::<Date32Array>()
            .unwrap();
        assert_eq!(datas.null_count(), 2);
        let valores = lote
            .column(6)
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .unwrap();
        assert_eq!(valores.null_count(), 0);
        let ordinais = lote
            .column(10)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap();
        let mut ordinais_publicados = ordinais.values().to_vec();
        ordinais_publicados.sort_unstable();
        assert_eq!(ordinais_publicados, [1, 2, 3, 4, 5]);
    }

    #[tokio::test]
    async fn despesas_contratadas_tse_2020_preserva_copias_exatas_e_sentinelas() {
        let bronze = Arc::new(Memoria::default());
        let snapshot = preparar_snapshot_despesas_contratadas(&bronze, 2020).await;
        let silver = Arc::new(Memoria::default());
        let resumo = publicar_tse_despesas_contratadas_2020(
            bronze.as_ref(),
            silver.as_ref(),
            &snapshot,
            Utc.with_ymd_and_hms(2026, 11, 10, 12, 30, 0).unwrap(),
            "silver-despesas-contratadas-2020",
        )
        .await
        .unwrap();
        assert_eq!(resumo.partes, 1);

        let bytes_ponteiro = silver
            .ler(&LocalizacaoRaw::ultima_versao(
                "tse",
                "prestacao-contas/ano=2020/despesas-contratadas",
            ))
            .await
            .unwrap()
            .unwrap();
        let ponteiro: UltimaVersao = serde_json::from_slice(&bytes_ponteiro).unwrap();
        let manifesto: ManifestoRaw =
            serde_json::from_slice(&silver.ler(&ponteiro.manifesto).await.unwrap().unwrap())
                .unwrap();
        let parquet = silver
            .ler(&manifesto.partes[0].chave)
            .await
            .unwrap()
            .unwrap();
        let leitor = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(parquet))
            .unwrap()
            .with_batch_size(10)
            .build()
            .unwrap();
        let lotes = leitor.map(Result::unwrap).collect::<Vec<_>>();
        assert_eq!(lotes.iter().map(|lote| lote.num_rows()).sum::<usize>(), 5);
        let lote = &lotes[0];

        let anos = lote
            .column(0)
            .as_any()
            .downcast_ref::<UInt16Array>()
            .unwrap();
        assert!(anos.values().iter().all(|ano| *ano == 2020));
        let sequencias = lote
            .column(3)
            .as_any()
            .downcast_ref::<UInt32Array>()
            .unwrap();
        assert_eq!(sequencias.null_count(), 2);
        assert_eq!(
            (0..sequencias.len())
                .filter(|indice| !sequencias.is_null(*indice) && sequencias.value(*indice) == 7)
                .count(),
            3
        );
        let datas = lote
            .column(4)
            .as_any()
            .downcast_ref::<Date32Array>()
            .unwrap();
        assert_eq!(datas.null_count(), 2);
        let valores = lote
            .column(6)
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .unwrap();
        assert_eq!(valores.null_count(), 0);
        let ordinais = lote
            .column(10)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap();
        let mut ordinais_publicados = ordinais.values().to_vec();
        ordinais_publicados.sort_unstable();
        assert_eq!(ordinais_publicados, [1, 2, 3, 4, 5]);
    }

    #[tokio::test]
    async fn despesas_contratadas_tse_2022_preserva_repeticoes_e_trata_sentinelas() {
        let bronze = Arc::new(Memoria::default());
        let snapshot = preparar_snapshot_despesas_contratadas(&bronze, 2022).await;
        let silver = Arc::new(Memoria::default());
        let resumo = publicar_tse_despesas_contratadas_2022(
            bronze.as_ref(),
            silver.as_ref(),
            &snapshot,
            Utc.with_ymd_and_hms(2026, 11, 10, 12, 30, 0).unwrap(),
            "silver-despesas-contratadas-2022",
        )
        .await
        .unwrap();
        assert_eq!(resumo.partes, 1);

        let bytes_ponteiro = silver
            .ler(&LocalizacaoRaw::ultima_versao(
                "tse",
                "prestacao-contas/ano=2022/despesas-contratadas",
            ))
            .await
            .unwrap()
            .unwrap();
        let ponteiro: UltimaVersao = serde_json::from_slice(&bytes_ponteiro).unwrap();
        let manifesto: ManifestoRaw =
            serde_json::from_slice(&silver.ler(&ponteiro.manifesto).await.unwrap().unwrap())
                .unwrap();
        let parquet = silver
            .ler(&manifesto.partes[0].chave)
            .await
            .unwrap()
            .unwrap();
        let leitor = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(parquet))
            .unwrap()
            .with_batch_size(10)
            .build()
            .unwrap();
        let lotes = leitor.map(Result::unwrap).collect::<Vec<_>>();
        assert_eq!(lotes.iter().map(|lote| lote.num_rows()).sum::<usize>(), 4);
        let lote = &lotes[0];
        let anos = lote
            .column(0)
            .as_any()
            .downcast_ref::<UInt16Array>()
            .unwrap();
        assert!(anos.values().iter().all(|ano| *ano == 2022));
        let sequencias = lote
            .column(3)
            .as_any()
            .downcast_ref::<UInt32Array>()
            .unwrap();
        assert_eq!(sequencias.null_count(), 2);
        assert_eq!(sequencias.value(0), 7);
        assert_eq!(sequencias.value(1), 7);
        let datas = lote
            .column(4)
            .as_any()
            .downcast_ref::<Date32Array>()
            .unwrap();
        assert_eq!(datas.null_count(), 2);
        let valores = lote
            .column(6)
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .unwrap();
        assert_eq!(valores.null_count(), 0);
        let ordinais = lote
            .column(10)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap();
        let mut ordinais_publicados = ordinais.values().to_vec();
        ordinais_publicados.sort_unstable();
        assert_eq!(ordinais_publicados, [1, 2, 3, 4]);
    }

    #[tokio::test]
    async fn receitas_doador_originario_tse_preserva_variantes_e_dados_ausentes() {
        let bronze = Arc::new(Memoria::default());
        let snapshot = preparar_snapshot_receitas_doador_originario(&bronze, 2024).await;
        let silver = Arc::new(Memoria::default());
        let resumo = publicar_tse_receitas_doador_originario_2024(
            bronze.as_ref(),
            silver.as_ref(),
            &snapshot,
            Utc.with_ymd_and_hms(2026, 11, 10, 12, 30, 0).unwrap(),
            "silver-receitas-doador-originario",
        )
        .await
        .unwrap();
        assert_eq!(resumo.partes, 1);

        let ponteiro = silver
            .ler(&LocalizacaoRaw::ultima_versao(
                "tse",
                "prestacao-contas/ano=2024/receitas-doador-originario",
            ))
            .await
            .unwrap()
            .unwrap();
        let ponteiro: UltimaVersao = serde_json::from_slice(&ponteiro).unwrap();
        let manifesto: ManifestoRaw =
            serde_json::from_slice(&silver.ler(&ponteiro.manifesto).await.unwrap().unwrap())
                .unwrap();
        let parquet = silver
            .ler(&manifesto.partes[0].chave)
            .await
            .unwrap()
            .unwrap();
        let leitor = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(parquet))
            .unwrap()
            .with_batch_size(10)
            .build()
            .unwrap();
        let lotes = leitor.map(Result::unwrap).collect::<Vec<_>>();
        assert_eq!(lotes.iter().map(|lote| lote.num_rows()).sum::<usize>(), 3);
        let lote = &lotes[0];

        let documentos = lote
            .column(3)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert_eq!(documentos.null_count(), 1);
        assert_eq!(documentos.value(0), "00123456789");
        assert_eq!(documentos.value(1), "00123456789");
        let nomes = lote
            .column(4)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert_eq!(nomes.value(0), "Doadora de teste");
        assert_eq!(nomes.value(1), "Doadora de teste retificado");
        for indice in [5, 6, 7, 8, 11] {
            assert_eq!(
                lote.column(indice)
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .unwrap()
                    .null_count(),
                1
            );
        }
        let sequencias = lote
            .column(9)
            .as_any()
            .downcast_ref::<UInt32Array>()
            .unwrap();
        assert_eq!(sequencias.null_count(), 1);
        assert_eq!(sequencias.value(0), 700);
        assert_eq!(sequencias.value(1), 700);
        let datas = lote
            .column(10)
            .as_any()
            .downcast_ref::<Date32Array>()
            .unwrap();
        assert_eq!(datas.null_count(), 1);
        let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
        let dia_10 = chrono::NaiveDate::from_ymd_opt(2024, 10, 10)
            .unwrap()
            .signed_duration_since(epoch)
            .num_days() as i32;
        assert_eq!(datas.value(0), dia_10);
        assert_eq!(datas.value(1), dia_10);
        let valores = lote
            .column(12)
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .unwrap();
        let mut centavos = (0..valores.len())
            .map(|i| valores.value(i))
            .collect::<Vec<_>>();
        centavos.sort_unstable();
        assert_eq!(centavos, [2_500, 10_000, 12_550]);
        let ordinais = lote
            .column(16)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap();
        let mut publicados = ordinais.values().to_vec();
        publicados.sort_unstable();
        assert_eq!(publicados, [1, 2, 3]);
        let originais = lote
            .column(13)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert!(
            (0..originais.len())
                .any(|i| originais.value(i).contains("Doadora de teste retificado"))
        );
    }

    #[tokio::test]
    async fn despesas_pagas_tse_2022_preserva_ocorrencias_e_tipa_campos() {
        let bronze = Arc::new(Memoria::default());
        let snapshot = preparar_snapshot_despesas_pagas(&bronze, 2022).await;
        let silver = Arc::new(Memoria::default());
        let resumo = publicar_tse_despesas_pagas_2022(
            bronze.as_ref(),
            silver.as_ref(),
            &snapshot,
            Utc.with_ymd_and_hms(2026, 11, 10, 12, 30, 0).unwrap(),
            "silver-despesas-pagas-2022",
        )
        .await
        .unwrap();
        assert_eq!(resumo.partes, 1);

        let bytes_ponteiro = silver
            .ler(&LocalizacaoRaw::ultima_versao(
                "tse",
                "prestacao-contas/ano=2022/despesas-pagas",
            ))
            .await
            .unwrap()
            .unwrap();
        let ponteiro: UltimaVersao = serde_json::from_slice(&bytes_ponteiro).unwrap();
        let manifesto: ManifestoRaw =
            serde_json::from_slice(&silver.ler(&ponteiro.manifesto).await.unwrap().unwrap())
                .unwrap();
        let parquet = silver
            .ler(&manifesto.partes[0].chave)
            .await
            .unwrap()
            .unwrap();
        let leitor = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(parquet))
            .unwrap()
            .with_batch_size(10)
            .build()
            .unwrap();
        let lotes = leitor.map(Result::unwrap).collect::<Vec<_>>();
        assert_eq!(lotes.iter().map(|lote| lote.num_rows()).sum::<usize>(), 3);
        let lote = &lotes[0];

        let anos = lote
            .column(0)
            .as_any()
            .downcast_ref::<UInt16Array>()
            .unwrap();
        assert!(anos.values().iter().all(|ano| *ano == 2022));
        let despesas = lote
            .column(3)
            .as_any()
            .downcast_ref::<UInt32Array>()
            .unwrap();
        assert!(despesas.values().iter().all(|sequencia| *sequencia == 200));
        let pagamentos = lote
            .column(4)
            .as_any()
            .downcast_ref::<UInt32Array>()
            .unwrap();
        assert_eq!(pagamentos.values(), &[1, 1, 2]);
        let datas = lote
            .column(5)
            .as_any()
            .downcast_ref::<Date32Array>()
            .unwrap();
        assert_eq!(datas.null_count(), 0);
        let valores = lote
            .column(6)
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .unwrap();
        let mut valores_centavos = valores.values().to_vec();
        valores_centavos.sort_unstable();
        assert_eq!(valores_centavos, [5_000, 12_550, 12_550]);
        let ordinais = lote
            .column(10)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap();
        let mut ordinais_publicados = ordinais.values().to_vec();
        ordinais_publicados.sort_unstable();
        assert_eq!(ordinais_publicados, [1, 2, 3]);
    }

    #[tokio::test]
    async fn despesas_pagas_tse_2018_preserva_ocorrencias_e_tipa_campos() {
        let bronze = Arc::new(Memoria::default());
        let snapshot = preparar_snapshot_despesas_pagas(&bronze, 2018).await;
        let silver = Arc::new(Memoria::default());
        let resumo = publicar_tse_despesas_pagas_2018(
            bronze.as_ref(),
            silver.as_ref(),
            &snapshot,
            Utc.with_ymd_and_hms(2026, 11, 11, 12, 30, 0).unwrap(),
            "silver-despesas-pagas-2018",
        )
        .await
        .unwrap();
        assert_eq!(resumo.partes, 1);

        let bytes_ponteiro = silver
            .ler(&LocalizacaoRaw::ultima_versao(
                "tse",
                "prestacao-contas/ano=2018/despesas-pagas",
            ))
            .await
            .unwrap()
            .unwrap();
        let ponteiro: UltimaVersao = serde_json::from_slice(&bytes_ponteiro).unwrap();
        let manifesto: ManifestoRaw =
            serde_json::from_slice(&silver.ler(&ponteiro.manifesto).await.unwrap().unwrap())
                .unwrap();
        let parquet = silver
            .ler(&manifesto.partes[0].chave)
            .await
            .unwrap()
            .unwrap();
        let leitor = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(parquet))
            .unwrap()
            .with_batch_size(10)
            .build()
            .unwrap();
        let lotes = leitor.map(Result::unwrap).collect::<Vec<_>>();
        assert_eq!(lotes.iter().map(|lote| lote.num_rows()).sum::<usize>(), 3);
        let lote = &lotes[0];

        let anos = lote
            .column(0)
            .as_any()
            .downcast_ref::<UInt16Array>()
            .unwrap();
        assert!(anos.values().iter().all(|ano| *ano == 2018));
        let despesas = lote
            .column(3)
            .as_any()
            .downcast_ref::<UInt32Array>()
            .unwrap();
        assert!(despesas.values().iter().all(|sequencia| *sequencia == 200));
        let pagamentos = lote
            .column(4)
            .as_any()
            .downcast_ref::<UInt32Array>()
            .unwrap();
        assert_eq!(pagamentos.values(), &[1, 1, 2]);
        let datas = lote
            .column(5)
            .as_any()
            .downcast_ref::<Date32Array>()
            .unwrap();
        assert_eq!(datas.null_count(), 0);
        let valores = lote
            .column(6)
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .unwrap();
        let mut valores_centavos = valores.values().to_vec();
        valores_centavos.sort_unstable();
        assert_eq!(valores_centavos, [5_000, 12_550, 12_550]);
        let ordinais = lote
            .column(10)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap();
        let mut ordinais_publicados = ordinais.values().to_vec();
        ordinais_publicados.sort_unstable();
        assert_eq!(ordinais_publicados, [1, 2, 3]);
    }

    #[tokio::test]
    async fn despesas_pagas_tse_2020_preserva_ocorrencias_e_tipa_campos() {
        let bronze = Arc::new(Memoria::default());
        let snapshot = preparar_snapshot_despesas_pagas(&bronze, 2020).await;
        let silver = Arc::new(Memoria::default());
        let resumo = publicar_tse_despesas_pagas_2020(
            bronze.as_ref(),
            silver.as_ref(),
            &snapshot,
            Utc.with_ymd_and_hms(2026, 11, 11, 12, 30, 0).unwrap(),
            "silver-despesas-pagas-2020",
        )
        .await
        .unwrap();
        assert_eq!(resumo.partes, 1);

        let bytes_ponteiro = silver
            .ler(&LocalizacaoRaw::ultima_versao(
                "tse",
                "prestacao-contas/ano=2020/despesas-pagas",
            ))
            .await
            .unwrap()
            .unwrap();
        let ponteiro: UltimaVersao = serde_json::from_slice(&bytes_ponteiro).unwrap();
        let manifesto: ManifestoRaw =
            serde_json::from_slice(&silver.ler(&ponteiro.manifesto).await.unwrap().unwrap())
                .unwrap();
        let parquet = silver
            .ler(&manifesto.partes[0].chave)
            .await
            .unwrap()
            .unwrap();
        let leitor = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(parquet))
            .unwrap()
            .with_batch_size(10)
            .build()
            .unwrap();
        let lotes = leitor.map(Result::unwrap).collect::<Vec<_>>();
        assert_eq!(lotes.iter().map(|lote| lote.num_rows()).sum::<usize>(), 3);
        let lote = &lotes[0];

        let anos = lote
            .column(0)
            .as_any()
            .downcast_ref::<UInt16Array>()
            .unwrap();
        assert!(anos.values().iter().all(|ano| *ano == 2020));
        let despesas = lote
            .column(3)
            .as_any()
            .downcast_ref::<UInt32Array>()
            .unwrap();
        assert!(despesas.values().iter().all(|sequencia| *sequencia == 200));
        let pagamentos = lote
            .column(4)
            .as_any()
            .downcast_ref::<UInt32Array>()
            .unwrap();
        assert_eq!(pagamentos.values(), &[1, 1, 2]);
        let datas = lote
            .column(5)
            .as_any()
            .downcast_ref::<Date32Array>()
            .unwrap();
        assert_eq!(datas.null_count(), 0);
        let valores = lote
            .column(6)
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .unwrap();
        let mut valores_centavos = valores.values().to_vec();
        valores_centavos.sort_unstable();
        assert_eq!(valores_centavos, [5_000, 12_550, 12_550]);
        let ordinais = lote
            .column(10)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap();
        let mut ordinais_publicados = ordinais.values().to_vec();
        ordinais_publicados.sort_unstable();
        assert_eq!(ordinais_publicados, [1, 2, 3]);
    }

    #[tokio::test]
    async fn receitas_doador_originario_tse_2022_seleciona_brasil_e_preserva_ocorrencias() {
        let bronze = Arc::new(Memoria::default());
        let snapshot = preparar_snapshot_receitas_doador_originario(&bronze, 2022).await;
        let silver = Arc::new(Memoria::default());
        let resumo = publicar_tse_receitas_doador_originario_2022(
            bronze.as_ref(),
            silver.as_ref(),
            &snapshot,
            Utc.with_ymd_and_hms(2026, 11, 10, 12, 30, 0).unwrap(),
            "silver-receitas-doador-originario-2022",
        )
        .await
        .unwrap();
        assert_eq!(resumo.partes, 1);

        let ponteiro = silver
            .ler(&LocalizacaoRaw::ultima_versao(
                "tse",
                "prestacao-contas/ano=2022/receitas-doador-originario",
            ))
            .await
            .unwrap()
            .unwrap();
        let ponteiro: UltimaVersao = serde_json::from_slice(&ponteiro).unwrap();
        let manifesto: ManifestoRaw =
            serde_json::from_slice(&silver.ler(&ponteiro.manifesto).await.unwrap().unwrap())
                .unwrap();
        let parquet = silver
            .ler(&manifesto.partes[0].chave)
            .await
            .unwrap()
            .unwrap();
        let leitor = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(parquet))
            .unwrap()
            .with_batch_size(10)
            .build()
            .unwrap();
        let lotes = leitor.map(Result::unwrap).collect::<Vec<_>>();
        assert_eq!(lotes.iter().map(|lote| lote.num_rows()).sum::<usize>(), 3);
        let lote = &lotes[0];
        let anos = lote
            .column(0)
            .as_any()
            .downcast_ref::<UInt16Array>()
            .unwrap();
        assert!(anos.values().iter().all(|ano| *ano == 2022));
        let sequencias = lote
            .column(9)
            .as_any()
            .downcast_ref::<UInt32Array>()
            .unwrap();
        assert_eq!(sequencias.null_count(), 1);
        assert_eq!(sequencias.value(0), 700);
        assert_eq!(sequencias.value(1), 700);
        let ordinais = lote
            .column(16)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap();
        let mut publicados = ordinais.values().to_vec();
        publicados.sort_unstable();
        assert_eq!(publicados, [1, 2, 3]);
    }

    #[tokio::test]
    async fn receitas_doador_originario_tse_2018_seleciona_brasil_e_preserva_ocorrencias() {
        let bronze = Arc::new(Memoria::default());
        let snapshot = preparar_snapshot_receitas_doador_originario(&bronze, 2018).await;
        let silver = Arc::new(Memoria::default());
        let resumo = publicar_tse_receitas_doador_originario_2018(
            bronze.as_ref(),
            silver.as_ref(),
            &snapshot,
            Utc.with_ymd_and_hms(2026, 11, 10, 12, 30, 0).unwrap(),
            "silver-receitas-doador-originario-2018",
        )
        .await
        .unwrap();
        assert_eq!(resumo.partes, 1);

        let ponteiro = silver
            .ler(&LocalizacaoRaw::ultima_versao(
                "tse",
                "prestacao-contas/ano=2018/receitas-doador-originario",
            ))
            .await
            .unwrap()
            .unwrap();
        let ponteiro: UltimaVersao = serde_json::from_slice(&ponteiro).unwrap();
        let manifesto: ManifestoRaw =
            serde_json::from_slice(&silver.ler(&ponteiro.manifesto).await.unwrap().unwrap())
                .unwrap();
        let parquet = silver
            .ler(&manifesto.partes[0].chave)
            .await
            .unwrap()
            .unwrap();
        let leitor = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(parquet))
            .unwrap()
            .with_batch_size(10)
            .build()
            .unwrap();
        let lotes = leitor.map(Result::unwrap).collect::<Vec<_>>();
        assert_eq!(lotes.iter().map(|lote| lote.num_rows()).sum::<usize>(), 4);
        let lote = &lotes[0];
        let anos = lote
            .column(0)
            .as_any()
            .downcast_ref::<UInt16Array>()
            .unwrap();
        assert!(anos.values().iter().all(|ano| *ano == 2018));
        let sequencias = lote
            .column(9)
            .as_any()
            .downcast_ref::<UInt32Array>()
            .unwrap();
        assert_eq!(sequencias.null_count(), 1);
        assert_eq!(sequencias.value(0), 700);
        assert_eq!(sequencias.value(1), 700);
        let ordinais = lote
            .column(16)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap();
        let mut publicados = ordinais.values().to_vec();
        publicados.sort_unstable();
        assert_eq!(publicados, [1, 2, 3, 4]);
        let originais = lote
            .column(13)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert_eq!(originais.value(0), originais.value(3));
    }

    #[tokio::test]
    async fn receitas_doador_originario_tse_2020_seleciona_brasil_e_preserva_ocorrencias() {
        let bronze = Arc::new(Memoria::default());
        let snapshot = preparar_snapshot_receitas_doador_originario(&bronze, 2020).await;
        let silver = Arc::new(Memoria::default());
        let resumo = publicar_tse_receitas_doador_originario_2020(
            bronze.as_ref(),
            silver.as_ref(),
            &snapshot,
            Utc.with_ymd_and_hms(2026, 11, 10, 12, 30, 0).unwrap(),
            "silver-receitas-doador-originario-2020",
        )
        .await
        .unwrap();
        assert_eq!(resumo.partes, 1);

        let ponteiro = silver
            .ler(&LocalizacaoRaw::ultima_versao(
                "tse",
                "prestacao-contas/ano=2020/receitas-doador-originario",
            ))
            .await
            .unwrap()
            .unwrap();
        let ponteiro: UltimaVersao = serde_json::from_slice(&ponteiro).unwrap();
        let manifesto: ManifestoRaw =
            serde_json::from_slice(&silver.ler(&ponteiro.manifesto).await.unwrap().unwrap())
                .unwrap();
        let parquet = silver
            .ler(&manifesto.partes[0].chave)
            .await
            .unwrap()
            .unwrap();
        let leitor = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(parquet))
            .unwrap()
            .with_batch_size(10)
            .build()
            .unwrap();
        let lotes = leitor.map(Result::unwrap).collect::<Vec<_>>();
        assert_eq!(lotes.iter().map(|lote| lote.num_rows()).sum::<usize>(), 4);
        let lote = &lotes[0];
        let anos = lote
            .column(0)
            .as_any()
            .downcast_ref::<UInt16Array>()
            .unwrap();
        assert!(anos.values().iter().all(|ano| *ano == 2020));
        let sequencias = lote
            .column(9)
            .as_any()
            .downcast_ref::<UInt32Array>()
            .unwrap();
        assert_eq!(sequencias.null_count(), 1);
        assert_eq!(sequencias.value(0), 700);
        assert_eq!(sequencias.value(1), 700);
        let ordinais = lote
            .column(16)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap();
        let mut publicados = ordinais.values().to_vec();
        publicados.sort_unstable();
        assert_eq!(publicados, [1, 2, 3, 4]);
        let originais = lote
            .column(13)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert_eq!(originais.value(0), originais.value(3));
    }

    #[tokio::test]
    async fn receitas_tse_preserva_variantes_e_tipa_sentinelas_e_valores() {
        let bronze = Arc::new(Memoria::default());
        let snapshot = preparar_snapshot_receitas(&bronze, 2024).await;
        let silver = Arc::new(Memoria::default());
        let resumo = publicar_tse_receitas_candidatos_2024(
            bronze.as_ref(),
            silver.as_ref(),
            &snapshot,
            Utc.with_ymd_and_hms(2026, 11, 10, 12, 30, 0).unwrap(),
            "silver-receitas",
        )
        .await
        .unwrap();
        assert_eq!(resumo.partes, 1);

        let bytes_ponteiro = silver
            .ler(&LocalizacaoRaw::ultima_versao(
                "tse",
                "prestacao-contas/ano=2024/receitas-candidatos",
            ))
            .await
            .unwrap()
            .unwrap();
        let ponteiro: UltimaVersao = serde_json::from_slice(&bytes_ponteiro).unwrap();
        let manifesto: ManifestoRaw =
            serde_json::from_slice(&silver.ler(&ponteiro.manifesto).await.unwrap().unwrap())
                .unwrap();
        let parquet = silver
            .ler(&manifesto.partes[0].chave)
            .await
            .unwrap()
            .unwrap();
        let leitor = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(parquet))
            .unwrap()
            .with_batch_size(10)
            .build()
            .unwrap();
        let lotes = leitor.map(Result::unwrap).collect::<Vec<_>>();
        assert_eq!(lotes.iter().map(|lote| lote.num_rows()).sum::<usize>(), 4);
        let lote = &lotes[0];
        assert_eq!(lote.num_rows(), 4);
        assert!(
            lote.column(0)
                .as_any()
                .downcast_ref::<UInt16Array>()
                .unwrap()
                .values()
                .iter()
                .all(|ano| *ano == 2024)
        );
        let sequencias = lote
            .column(3)
            .as_any()
            .downcast_ref::<UInt32Array>()
            .unwrap();
        assert_eq!(sequencias.null_count(), 2);
        let mut sequencias_validas = (0..sequencias.len())
            .filter(|indice| !sequencias.is_null(*indice))
            .map(|indice| sequencias.value(indice))
            .collect::<Vec<_>>();
        sequencias_validas.sort_unstable();
        assert_eq!(sequencias_validas, [500, 500]);
        let datas = lote
            .column(4)
            .as_any()
            .downcast_ref::<Date32Array>()
            .unwrap();
        assert_eq!(datas.null_count(), 1);
        let mut datas_desde_epoch = (0..datas.len())
            .filter(|indice| !datas.is_null(*indice))
            .map(|indice| datas.value(indice))
            .collect::<Vec<_>>();
        datas_desde_epoch.sort_unstable();
        let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
        let dia_15 = chrono::NaiveDate::from_ymd_opt(2024, 11, 15)
            .unwrap()
            .signed_duration_since(epoch)
            .num_days() as i32;
        let dia_16 = chrono::NaiveDate::from_ymd_opt(2024, 11, 16)
            .unwrap()
            .signed_duration_since(epoch)
            .num_days() as i32;
        assert_eq!(datas_desde_epoch, [dia_15, dia_16, dia_16]);
        let valores = lote
            .column(5)
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .unwrap();
        let mut valores_centavos = (0..valores.len())
            .map(|indice| valores.value(indice))
            .collect::<Vec<_>>();
        valores_centavos.sort_unstable();
        assert_eq!(valores_centavos, [1_550, 12_345, 25_000, 27_550]);
        let ordinais = lote
            .column(9)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap();
        let mut ordinais_publicados = ordinais.values().to_vec();
        ordinais_publicados.sort_unstable();
        assert_eq!(ordinais_publicados, [1, 2, 3, 4]);
        let originais = lote
            .column(6)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let conteudo_original = (0..originais.len())
            .map(|indice| originais.value(indice))
            .collect::<Vec<_>>();
        assert!(
            conteudo_original
                .iter()
                .any(|linha| linha.contains("doação estimável retificada"))
        );
    }

    #[tokio::test]
    async fn receitas_candidatos_tse_2022_preserva_repeticoes_e_tipa_sentinelas() {
        let bronze = Arc::new(Memoria::default());
        let snapshot = preparar_snapshot_receitas(&bronze, 2022).await;
        let silver = Arc::new(Memoria::default());
        let resumo = publicar_tse_receitas_candidatos_2022(
            bronze.as_ref(),
            silver.as_ref(),
            &snapshot,
            Utc.with_ymd_and_hms(2026, 11, 10, 12, 30, 0).unwrap(),
            "silver-receitas-2022",
        )
        .await
        .unwrap();
        assert_eq!(resumo.partes, 1);

        let bytes_ponteiro = silver
            .ler(&LocalizacaoRaw::ultima_versao(
                "tse",
                "prestacao-contas/ano=2022/receitas-candidatos",
            ))
            .await
            .unwrap()
            .unwrap();
        let ponteiro: UltimaVersao = serde_json::from_slice(&bytes_ponteiro).unwrap();
        let manifesto: ManifestoRaw =
            serde_json::from_slice(&silver.ler(&ponteiro.manifesto).await.unwrap().unwrap())
                .unwrap();
        let parquet = silver
            .ler(&manifesto.partes[0].chave)
            .await
            .unwrap()
            .unwrap();
        let leitor = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(parquet))
            .unwrap()
            .with_batch_size(10)
            .build()
            .unwrap();
        let lotes = leitor.map(Result::unwrap).collect::<Vec<_>>();
        assert_eq!(lotes.iter().map(|lote| lote.num_rows()).sum::<usize>(), 4);
        let lote = &lotes[0];
        assert_eq!(lote.num_rows(), 4);
        let anos = lote
            .column(0)
            .as_any()
            .downcast_ref::<UInt16Array>()
            .unwrap();
        assert!(anos.values().iter().all(|ano| *ano == 2022));
        let sequencias = lote
            .column(3)
            .as_any()
            .downcast_ref::<UInt32Array>()
            .unwrap();
        assert_eq!(sequencias.null_count(), 2);
        assert_eq!(sequencias.value(1), 500);
        assert_eq!(sequencias.value(2), 500);
        let datas = lote
            .column(4)
            .as_any()
            .downcast_ref::<Date32Array>()
            .unwrap();
        assert_eq!(datas.null_count(), 1);
        let ordinais = lote
            .column(9)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap();
        let mut ordinais_publicados = ordinais.values().to_vec();
        ordinais_publicados.sort_unstable();
        assert_eq!(ordinais_publicados, [1, 2, 3, 4]);
    }

    #[tokio::test]
    async fn receitas_candidatos_tse_2018_preserva_repeticoes_e_tipa_sentinelas() {
        let bronze = Arc::new(Memoria::default());
        let snapshot = preparar_snapshot_receitas(&bronze, 2018).await;
        let silver = Arc::new(Memoria::default());
        let resumo = publicar_tse_receitas_candidatos_2018(
            bronze.as_ref(),
            silver.as_ref(),
            &snapshot,
            Utc.with_ymd_and_hms(2026, 11, 10, 12, 30, 0).unwrap(),
            "silver-receitas-2018",
        )
        .await
        .unwrap();
        assert_eq!(resumo.partes, 1);

        let bytes_ponteiro = silver
            .ler(&LocalizacaoRaw::ultima_versao(
                "tse",
                "prestacao-contas/ano=2018/receitas-candidatos",
            ))
            .await
            .unwrap()
            .unwrap();
        let ponteiro: UltimaVersao = serde_json::from_slice(&bytes_ponteiro).unwrap();
        let manifesto: ManifestoRaw =
            serde_json::from_slice(&silver.ler(&ponteiro.manifesto).await.unwrap().unwrap())
                .unwrap();
        let parquet = silver
            .ler(&manifesto.partes[0].chave)
            .await
            .unwrap()
            .unwrap();
        let leitor = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(parquet))
            .unwrap()
            .with_batch_size(10)
            .build()
            .unwrap();
        let lotes = leitor.map(Result::unwrap).collect::<Vec<_>>();
        assert_eq!(lotes.iter().map(|lote| lote.num_rows()).sum::<usize>(), 5);
        let lote = &lotes[0];
        assert_eq!(lote.num_rows(), 5);
        let anos = lote
            .column(0)
            .as_any()
            .downcast_ref::<UInt16Array>()
            .unwrap();
        assert!(anos.values().iter().all(|ano| *ano == 2018));
        let sequencias = lote
            .column(3)
            .as_any()
            .downcast_ref::<UInt32Array>()
            .unwrap();
        assert_eq!(sequencias.null_count(), 2);
        assert_eq!(sequencias.value(1), 500);
        assert_eq!(sequencias.value(2), 500);
        let datas = lote
            .column(4)
            .as_any()
            .downcast_ref::<Date32Array>()
            .unwrap();
        assert_eq!(datas.null_count(), 1);
        let ordinais = lote
            .column(9)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap();
        let mut ordinais_publicados = ordinais.values().to_vec();
        ordinais_publicados.sort_unstable();
        assert_eq!(ordinais_publicados, [1, 2, 3, 4, 5]);
    }

    #[tokio::test]
    async fn receitas_candidatos_tse_2020_preserva_repeticoes_e_tipa_sentinelas() {
        let bronze = Arc::new(Memoria::default());
        let snapshot = preparar_snapshot_receitas(&bronze, 2020).await;
        let silver = Arc::new(Memoria::default());
        let resumo = publicar_tse_receitas_candidatos_2020(
            bronze.as_ref(),
            silver.as_ref(),
            &snapshot,
            Utc.with_ymd_and_hms(2026, 11, 10, 12, 30, 0).unwrap(),
            "silver-receitas-2020",
        )
        .await
        .unwrap();
        assert_eq!(resumo.partes, 1);

        let bytes_ponteiro = silver
            .ler(&LocalizacaoRaw::ultima_versao(
                "tse",
                "prestacao-contas/ano=2020/receitas-candidatos",
            ))
            .await
            .unwrap()
            .unwrap();
        let ponteiro: UltimaVersao = serde_json::from_slice(&bytes_ponteiro).unwrap();
        let manifesto: ManifestoRaw =
            serde_json::from_slice(&silver.ler(&ponteiro.manifesto).await.unwrap().unwrap())
                .unwrap();
        let parquet = silver
            .ler(&manifesto.partes[0].chave)
            .await
            .unwrap()
            .unwrap();
        let leitor = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(parquet))
            .unwrap()
            .with_batch_size(10)
            .build()
            .unwrap();
        let lotes = leitor.map(Result::unwrap).collect::<Vec<_>>();
        assert_eq!(lotes.iter().map(|lote| lote.num_rows()).sum::<usize>(), 6);
        let lote = &lotes[0];
        assert_eq!(lote.num_rows(), 6);
        let anos = lote
            .column(0)
            .as_any()
            .downcast_ref::<UInt16Array>()
            .unwrap();
        assert!(anos.values().iter().all(|ano| *ano == 2020));
        let sequencias = lote
            .column(3)
            .as_any()
            .downcast_ref::<UInt32Array>()
            .unwrap();
        assert_eq!(sequencias.null_count(), 4);
        assert_eq!(sequencias.value(1), 500);
        assert_eq!(sequencias.value(2), 500);
        let datas = lote
            .column(4)
            .as_any()
            .downcast_ref::<Date32Array>()
            .unwrap();
        assert_eq!(datas.null_count(), 3);
        let ordinais = lote
            .column(9)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap();
        let mut ordinais_publicados = ordinais.values().to_vec();
        ordinais_publicados.sort_unstable();
        assert_eq!(ordinais_publicados, [1, 2, 3, 4, 5, 6]);
    }

    #[tokio::test]
    async fn executor_emite_multiplos_batches_sem_materializar_o_resultado() {
        let bronze = Arc::new(Memoria::default());
        let snapshot = preparar_snapshot(&bronze).await;
        let entrada = preparar_entrada_sql(bronze.as_ref(), &snapshot, 1_000_000)
            .await
            .unwrap();
        let spill = entrada.diretorio_execucao().join("spill-teste");
        std::fs::create_dir(&spill).unwrap();
        let mut fluxo = executar_sql(
            entrada.diretorio_bronze(),
            &spill,
            SQL,
            &ConfiguracaoExecutorSql {
                limite_entrada_bytes: 1_000_000,
                limite_memoria_bytes: 64 * 1024 * 1024,
                limite_spill_bytes: 64 * 1024 * 1024,
                particoes: 1,
                tamanho_lote: 1,
            },
        )
        .await
        .unwrap();
        let mut lotes = 0;
        let mut linhas = 0;
        while let Some(lote) = fluxo.next().await {
            lotes += 1;
            linhas += lote.unwrap().num_rows();
        }
        assert!(lotes > 1);
        assert_eq!(linhas, 3);
    }

    #[tokio::test]
    async fn falha_sql_preserva_o_ponteiro_silver_vigente() {
        let bronze = Arc::new(Memoria::default());
        let snapshot = Arc::new(preparar_snapshot(&bronze).await);
        let silver = Arc::new(Memoria::default());
        let tarefa_valida = nova_tarefa(Arc::clone(&bronze), Arc::clone(&snapshot), REGRA, SQL);
        tarefa_valida
            .executar(&contexto(Arc::clone(&silver), "silver-1"))
            .await
            .unwrap();
        let chave_ponteiro = LocalizacaoRaw::ultima_versao(FONTE, RECURSO);
        let ponteiro_anterior = silver.ler(&chave_ponteiro).await.unwrap().unwrap();

        let tarefa_invalida = nova_tarefa(
            Arc::clone(&bronze),
            snapshot,
            "teste-v2",
            "SELECT coluna_inexistente FROM bronze",
        );
        assert!(
            tarefa_invalida
                .executar(&contexto(Arc::clone(&silver), "silver-2"))
                .await
                .is_err()
        );
        assert_eq!(
            silver.ler(&chave_ponteiro).await.unwrap().unwrap(),
            ponteiro_anterior
        );
    }

    #[tokio::test]
    async fn destino_distingue_tarefas_irmas_no_dag() {
        let bronze = Arc::new(Memoria::default());
        let snapshot = Arc::new(preparar_snapshot(&bronze).await);
        let origem = nova_tarefa(Arc::clone(&bronze), Arc::clone(&snapshot), REGRA, SQL);
        let destino = TarefaDeSilverSql::nova(
            bronze,
            snapshot,
            FONTE,
            RECURSO,
            Some("outra-tabela".into()),
            REGRA,
            SQL,
            esquema_saida(),
        );
        assert_eq!(origem.nome(), RECURSO);
        assert_eq!(destino.nome(), "outra-tabela");
    }

    #[tokio::test]
    async fn ponteiro_mais_novo_recusado_nao_informa_publicacao() {
        let bronze = Arc::new(Memoria::default());
        let snapshot = Arc::new(preparar_snapshot(&bronze).await);
        let silver = Arc::new(Memoria::default());
        let ponteiro = UltimaVersao {
            versao: Some("versao-mais-nova".into()),
            manifesto: "manifesto-mais-novo".into(),
            iniciada_em: Some(Utc.with_ymd_and_hms(2026, 11, 6, 13, 0, 0).unwrap()),
        };
        let chave = LocalizacaoRaw::ultima_versao(FONTE, RECURSO);
        let bytes = serde_json::to_vec(&ponteiro).unwrap();
        silver
            .gravar(ObjetoRaw {
                chave: chave.clone(),
                conteudo: bytes.clone(),
                tipo_de_conteudo: "application/json".into(),
            })
            .await
            .unwrap();
        let tarefa = nova_tarefa(bronze, snapshot, REGRA, SQL);
        let erro = tarefa
            .executar(&contexto(Arc::clone(&silver), "silver-antiga"))
            .await
            .unwrap_err();
        assert!(erro.to_string().contains("não foi publicada"));
        assert_eq!(silver.ler(&chave).await.unwrap().unwrap(), bytes);
    }

    #[tokio::test]
    async fn linhas_duplicadas_no_bronze_passam_para_silver_com_linhagem_distinta() {
        let bronze = Arc::new(Memoria::default());
        let instante = Utc.with_ymd_and_hms(2026, 11, 6, 12, 0, 0).unwrap();
        let local = LocalizacaoRaw::nova(FONTE, RECURSO, instante, "bronze-dup");
        let conteudo = parquet(&[("5", "50"), ("5", "50"), ("6", "60")]);
        let parte = ParteRaw {
            chave: local.parte(1, "parquet"),
            url: "parte-dup".into(),
            bytes: conteudo.len() as u64,
            sha256: hex::encode(Sha256::digest(&conteudo)),
            tipo_de_conteudo: "application/vnd.apache.parquet".into(),
            versao: Some("bronze-dup".into()),
        };
        bronze
            .gravar(ObjetoRaw {
                chave: parte.chave.clone(),
                conteudo,
                tipo_de_conteudo: "application/vnd.apache.parquet".into(),
            })
            .await
            .unwrap();
        let manifesto = ManifestoRaw {
            fonte: FONTE.into(),
            recurso: RECURSO.into(),
            id_execucao: "bronze-dup".into(),
            url_inicial: "fixture-dup".into(),
            iniciada_em: instante,
            concluida_em: instante,
            partes: vec![parte],
        };
        let chave_manifesto = local.manifesto();
        bronze
            .gravar(ObjetoRaw {
                chave: chave_manifesto.clone(),
                conteudo: serde_json::to_vec(&manifesto).unwrap(),
                tipo_de_conteudo: "application/json".into(),
            })
            .await
            .unwrap();
        bronze
            .gravar(ObjetoRaw {
                chave: LocalizacaoRaw::ultima_versao(FONTE, RECURSO),
                conteudo: serde_json::to_vec(&UltimaVersao {
                    versao: Some("bronze-dup".into()),
                    manifesto: chave_manifesto,
                    iniciada_em: Some(instante),
                })
                .unwrap(),
                tipo_de_conteudo: "application/json".into(),
            })
            .await
            .unwrap();
        let snapshot = Arc::new(
            SnapshotBronze::vigente(bronze.as_ref(), FONTE, RECURSO)
                .await
                .unwrap()
                .unwrap(),
        );
        let sql_dup = "SELECT CAST(a.id AS BIGINT) AS id, CAST(a.valor AS BIGINT) AS valor, a._tdoa_origem_manifesto AS _tdoa_origem_manifesto, a._tdoa_origem_parte AS _tdoa_origem_parte, a._tdoa_origem_ordinal AS _tdoa_origem_ordinal FROM bronze AS a";
        let silver = Arc::new(Memoria::default());
        let tarefa = nova_tarefa(Arc::clone(&bronze), snapshot, "dup-v1", sql_dup);
        tarefa
            .executar(&contexto(Arc::clone(&silver), "silver-dup"))
            .await
            .unwrap();

        let bytes_ponteiro = silver
            .ler(&LocalizacaoRaw::ultima_versao(FONTE, RECURSO))
            .await
            .unwrap()
            .unwrap();
        let ponteiro: UltimaVersao = serde_json::from_slice(&bytes_ponteiro).unwrap();
        let manifesto_silver: ManifestoRaw =
            serde_json::from_slice(&silver.ler(&ponteiro.manifesto).await.unwrap().unwrap())
                .unwrap();
        let leitor = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(
            silver
                .ler(&manifesto_silver.partes[0].chave)
                .await
                .unwrap()
                .unwrap(),
        ))
        .unwrap()
        .build()
        .unwrap();
        let mut ordinais = Vec::new();
        for lote in leitor {
            let lote = lote.unwrap();
            ordinais.extend(
                lote.column(4)
                    .as_any()
                    .downcast_ref::<UInt64Array>()
                    .unwrap()
                    .values()
                    .iter()
                    .copied(),
            );
        }
        assert_eq!(
            ordinais,
            [1, 2, 3],
            "duplicatas Bronze preservam ordinais distintos"
        );
    }

    #[tokio::test]
    async fn race_bronze_silver_rejeita_publicacao_quando_ponteiro_bronze_muda() {
        let bronze = Arc::new(Memoria::default());
        let snapshot = Arc::new(preparar_snapshot(&bronze).await);
        let silver = Arc::new(Memoria::default());

        let controle = Arc::new(ControleUpload::default());
        *silver.bloqueio_upload.lock().unwrap() = Some(Arc::clone(&controle));

        let tarefa = nova_tarefa(Arc::clone(&bronze), Arc::clone(&snapshot), REGRA, SQL);
        let bronze_clone = Arc::clone(&bronze);
        let execucao =
            tokio::spawn(async move { tarefa.executar(&contexto(silver, "silver-race")).await });

        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            controle.iniciou.notified(),
        )
        .await
        .expect("a tarefa deveria chegar ao upload do Parquet");

        let instante_novo = Utc.with_ymd_and_hms(2026, 11, 7, 0, 0, 0).unwrap();
        let novo_ponteiro = UltimaVersao {
            versao: Some("bronze-2".into()),
            manifesto: "manifesto-novo".into(),
            iniciada_em: Some(instante_novo),
        };
        bronze_clone
            .gravar(ObjetoRaw {
                chave: LocalizacaoRaw::ultima_versao(FONTE, RECURSO),
                conteudo: serde_json::to_vec(&novo_ponteiro).unwrap(),
                tipo_de_conteudo: "application/json".into(),
            })
            .await
            .unwrap();

        controle.continuar.notify_one();
        let resultado = execucao.await.unwrap();
        assert!(
            resultado.is_err(),
            "Silver deve falhar quando o ponteiro Bronze mudou durante a transformação"
        );
    }

    #[tokio::test]
    async fn cancelamento_durante_upload_preserva_o_ponteiro_silver_vigente() {
        let bronze = Arc::new(Memoria::default());
        let snapshot = Arc::new(preparar_snapshot(&bronze).await);
        let silver = Arc::new(Memoria::default());
        let tarefa_valida = nova_tarefa(Arc::clone(&bronze), Arc::clone(&snapshot), REGRA, SQL);
        tarefa_valida
            .executar(&contexto(Arc::clone(&silver), "silver-1"))
            .await
            .unwrap();
        let chave_ponteiro = LocalizacaoRaw::ultima_versao(FONTE, RECURSO);
        let ponteiro_anterior = silver.ler(&chave_ponteiro).await.unwrap().unwrap();

        let controle = Arc::new(ControleUpload::default());
        *silver.bloqueio_upload.lock().unwrap() = Some(Arc::clone(&controle));
        let tarefa_reprocessamento = nova_tarefa(Arc::clone(&bronze), snapshot, "teste-v2", SQL);
        let silver_execucao = Arc::clone(&silver);
        let execucao = tokio::spawn(async move {
            tarefa_reprocessamento
                .executar(&contexto(silver_execucao, "silver-2"))
                .await
        });
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            controle.iniciou.notified(),
        )
        .await
        .expect("a tarefa deveria chegar ao upload do Parquet");
        execucao.abort();
        assert!(execucao.await.unwrap_err().is_cancelled());
        assert_eq!(
            silver.ler(&chave_ponteiro).await.unwrap().unwrap(),
            ponteiro_anterior
        );
    }

    #[test]
    fn nenhum_sql_silver_orfao_nem_contrato_sem_consulta() {
        let dir_sql = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("sql/silver");
        let dir_src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/silver");

        let sqls_no_disco: std::collections::BTreeSet<String> = std::fs::read_dir(&dir_sql)
            .unwrap()
            .filter_map(|e| {
                let nome = e.ok()?.file_name().to_str()?.to_owned();
                nome.ends_with(".sql").then_some(nome)
            })
            .collect();

        let mut sqls_referenciados = std::collections::BTreeSet::new();
        for entrada in std::fs::read_dir(&dir_src).unwrap() {
            let caminho = entrada.unwrap().path();
            if caminho.extension().is_some_and(|ext| ext == "rs") {
                let conteudo = std::fs::read_to_string(&caminho).unwrap();
                for linha in conteudo.lines() {
                    let padrao = "include_str!(\"../../sql/silver/";
                    if let Some(pos) = linha.find(padrao) {
                        let resto = &linha[pos + padrao.len()..];
                        if let Some(fim) = resto.find('"') {
                            sqls_referenciados.insert(resto[..fim].to_owned());
                        }
                    }
                }
            }
        }

        let orfaos: Vec<_> = sqls_no_disco.difference(&sqls_referenciados).collect();
        assert!(
            orfaos.is_empty(),
            "SQLs no disco sem include_str! correspondente: {orfaos:?}"
        );

        let fantasmas: Vec<_> = sqls_referenciados.difference(&sqls_no_disco).collect();
        assert!(
            fantasmas.is_empty(),
            "include_str! referencia SQLs inexistentes: {fantasmas:?}"
        );
    }
}
