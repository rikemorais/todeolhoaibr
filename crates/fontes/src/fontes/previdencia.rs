use std::time::Duration;

use todeolho_nucleo::{
    FonteDeDados, LimiteHttp, PerfilHttp, RedirecionamentoHttp, TarefaPlanejada,
};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "previdencia";

const BASE_S3: &str = "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com";

const PRIMEIRO_ANO: u16 = 2023;
const PRIMEIRO_MES: u8 = 6;
const ULTIMO_ANO: u16 = 2026;
const ULTIMO_MES_EMITIDOS: u8 = 9;
const ULTIMO_MES_MANTIDOS: u8 = 9;

/// URLs do dataset de concedidos extraídas da API CKAN do INSS.
///
/// Os nomes de arquivo são imprevisíveis (cada mês usa uma convenção
/// diferente), por isso armazenamos as URLs completas do S3.
const CONCEDIDOS: &[(&str, &str)] = &[
    (
        "202306",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2023_2025/Grupos_de_dados/Benef%C3%ADcios+concedidos/JUN23-ABR24/DADOS+ABERTOS_CONCEDIDOS_JUNHO+2023.xlsx",
    ),
    (
        "202307",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2023_2025/Grupos_de_dados/Benef%C3%ADcios+concedidos/JUN23-ABR24/DADOS+ABERTOS_CONCEDIDOS_JULHO+2023.xlsx",
    ),
    (
        "202308",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2023_2025/Grupos_de_dados/Benef%C3%ADcios+concedidos/JUN23-ABR24/DADOS+ABERTOS_CONCEDIDOS_AGOSTO+2023.xlsx",
    ),
    (
        "202309",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2023_2025/Grupos_de_dados/Benef%C3%ADcios+concedidos/JUN23-ABR24/DADOS+ABERTOS_CONCEDIDOS_SETEMBRO+2023.xlsx",
    ),
    (
        "202310",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2023_2025/Grupos_de_dados/Benef%C3%ADcios+concedidos/JUN23-ABR24/DADOS+ABERTOS_CONCEDIDOS_OUTUBRO+2023.xlsx",
    ),
    (
        "202311",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2023_2025/Grupos_de_dados/Benef%C3%ADcios+concedidos/JUN23-ABR24/DADOS+ABERTOS_CONCEDIDOS_NOVEMBRO+2023.xlsx",
    ),
    (
        "202312",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2023_2025/Grupos_de_dados/Benef%C3%ADcios+concedidos/JUN23-ABR24/DADOS+ABERTOS_CONCEDIDOS_DEZEMBRO+2023.xlsx",
    ),
    (
        "202401",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2023_2025/Grupos_de_dados/Benef%C3%ADcios+concedidos/JUN23-ABR24/DADOS+ABERTOS_CONCEDIDOS_JANEIRO+2024.xlsx",
    ),
    (
        "202402",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2023_2025/Grupos_de_dados/Benef%C3%ADcios+concedidos/JUN23-ABR24/DADOS+ABERTOS_CONCEDIDOS_FEVEREIRO+2024.xlsx",
    ),
    (
        "202403",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2023_2025/Grupos_de_dados/Benef%C3%ADcios+concedidos/JUN23-ABR24/DADOS+ABERTOS_CONCEDIDOS_MAR%C3%87O+2024.xlsx",
    ),
    (
        "202404",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2023_2025/Grupos_de_dados/Benef%C3%ADcios+concedidos/JUN23-ABR24/DADOS+ABERTOS_CONCEDIDOS_ABRIL+2024.xlsx",
    ),
    (
        "202405",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2023_2025/Grupos_de_dados/Benef%C3%ADcios+concedidos/DADOS+ABERTOS_CONCEDIDOS_MAIO+2024.xlsx",
    ),
    (
        "202406",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2023_2025/Grupos_de_dados/Benef%C3%ADcios+concedidos/CONCEDIDOS+JUNHO_DADOS+ABERTOS.xlsx",
    ),
    (
        "202407",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2023_2025/Grupos_de_dados/Benef%C3%ADcios+concedidos/CONCEDIDOS_DADOS+ABERTOS_M%C3%8AS+DE+JULHO+2024_FONTE+SUIBE_12.08.2024_consulta24823958.xlsx",
    ),
    (
        "202408",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2023_2025/Grupos_de_dados/Benef%C3%ADcios+concedidos/CONCEDIDOS+REGISTRO_DADOS+ABERTOS_COMPET%C3%8ANCIA+AGOSTO+DE+2024_SUIBE_consulta60939491.xlsx",
    ),
    (
        "202409",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2023_2025/Grupos_de_dados/Benef%C3%ADcios+concedidos/CONCEDIDOS_SETEMBRO+2024_DADOS+ABERTOS_SUIBE.xlsx",
    ),
    (
        "202410",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2023_2025/Grupos_de_dados/Benef%C3%ADcios+concedidos/CONCEDIDOS_OUTUBRO+2024_DADOS+ABERTOS_SUIBE.xlsx",
    ),
    (
        "202411",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2023_2025/Grupos_de_dados/Benef%C3%ADcios+concedidos/CONCEDIDOS_DADOS+ABERTOS_NOVEMBRO+2024_SUIBE_consulta27432795.xlsx",
    ),
    (
        "202412",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2023_2025/Grupos_de_dados/Benef%C3%ADcios+concedidos/CONCEDIDOS+DADOS+ABERTOS_DEZEMBRO+DE+2024_SUIBE_consulta59807528.xlsx",
    ),
    (
        "202501",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2023_2025/Grupos_de_dados/Benef%C3%ADcios+concedidos/CONCEDIDOS_DADOS_ABERTOS_JANEIRO+2025.xlsx",
    ),
    (
        "202502",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2023_2025/Grupos_de_dados/Benef%C3%ADcios+concedidos/CONCEDIDOS_DADOS+ABERTOS_FEVEREIRO+2025.xlsx",
    ),
    (
        "202503",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2023_2025/Grupos_de_dados/Benef%C3%ADcios+concedidos/BEN_CONCEDIDOS_032025.xlsx",
    ),
    (
        "202504",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2023_2025/Grupos_de_dados/Benef%C3%ADcios+concedidos/BEN_CONCEDIDOS_042025.xlsx",
    ),
    (
        "202505",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2023_2025/Grupos_de_dados/Benef%C3%ADcios+concedidos/CONCEDIDOS+DADOS+ABERTOS+MAIO+2025.xlsx",
    ),
    (
        "202506",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2023_2025/Grupos_de_dados/Benef%C3%ADcios+concedidos/CONCEDIDOS+DADOS+ABERTOS+JUNHO+2025.xlsx",
    ),
    (
        "202507",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2025_2027/Grupos_de_dados/Benef%C3%ADcios+concedidos/CONCEDIDOS+DADOS+ABERTOS_JULHO+2025.xlsx",
    ),
    (
        "202508",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2025_2027/Grupos_de_dados/Benef%C3%ADcios+concedidos/CONCEDIDOS_DADOS+ABERTOS_AGOSTO+DE+2025.xlsx",
    ),
    (
        "202509",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2025_2027/Grupos_de_dados/Benef%C3%ADcios+concedidos/CONCEDIDOS+DADOS+ABERTOS_SETEMBRO+2025.xlsx",
    ),
    (
        "202510",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2025_2027/Grupos_de_dados/Benef%C3%ADcios+concedidos/CONCEDIDOS+DADOS+ABERTOS_OUTUBRO+2025.xlsx",
    ),
    (
        "202511",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2025_2027/Grupos_de_dados/Benef%C3%ADcios+concedidos/DADOS+ABERTOS_CONCEDIDOS_NOVEMBRO+2025.xlsx",
    ),
    (
        "202512",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2025_2027/Grupos_de_dados/Benef%C3%ADcios+concedidos/BEN_CONCEDIDOS_122025.xlsx",
    ),
    (
        "202601",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2025_2027/Grupos_de_dados/Benef%C3%ADcios+concedidos/CONCEDIDOS_DADOS_ABERTOS_JANEIRO+2026.xlsx",
    ),
    (
        "202602",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2025_2027/Grupos_de_dados/Benef%C3%ADcios+concedidos/CONCEDIDOS_DADOS_ABERTOS_FEVEREIRO+2026.xlsx",
    ),
    (
        "202603",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2025_2027/Grupos_de_dados/Benef%C3%ADcios+concedidos/CONCEDIDOS_DADOS+ABERTOS_MAR%C3%87O+DE+2026.xlsx",
    ),
    (
        "202604",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2025_2027/Grupos_de_dados/Benef%C3%ADcios+concedidos/CONCEDIDOS_DADOS+ABERTOS_ABRIL+2026.xlsx",
    ),
    (
        "202605",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2025_2027/Grupos_de_dados/Benef%C3%ADcios+concedidos/DADOS+ABERTOS_CONCEDIDOS_MAIO+DE+2026.xlsx",
    ),
    (
        "202606",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2025_2027/Grupos_de_dados/Benef%C3%ADcios+concedidos/CONCEDIDOS_DADOS+ABERTOS_JUN+2026.xlsx",
    ),
    (
        "202607",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2025_2027/Grupos_de_dados/Benef%C3%ADcios+concedidos/CONCEDIDOS-DADOS+ABERTOS_JULHO+2026.xlsx",
    ),
    (
        "202608",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2025_2027/Grupos_de_dados/Benef%C3%ADcios+concedidos/CONCEDIDOS+DADOS+ABERTOS+AGOSTO_2026.xlsx",
    ),
    (
        "202609",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2025_2027/Grupos_de_dados/Benef%C3%ADcios+concedidos/CONCEDIDOS_DADOS+ABERTOS_SETEMBRO+2026.xlsx",
    ),
    (
        "202610",
        "https://armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com/PDA_2025_2027/Grupos_de_dados/Benef%C3%ADcios+concedidos/CONCEDIDOS_DADOS+ABERTOS_OUTUBRO+2026.xlsx",
    ),
];

/// Benefícios previdenciários do RGPS/INSS — dados abertos do INSS.
///
/// Três datasets mensais do Portal de Dados Abertos do INSS:
/// - **Concedidos**: XLSX com competência, espécie, CID, clientela, município
/// - **Emitidos**: ZIP/CSV com espécie, UF, município, valor líquido
/// - **Mantidos**: ZIP/CSV por status (ativos, cessados, suspensos)
pub struct FontePrevidencia;

impl FonteDeDados for FontePrevidencia {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Benefícios previdenciários do RGPS/INSS — dados abertos mensais"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            credencial: None,
            limite: Some(LimiteHttp {
                host: "armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com",
                requisicoes: 4,
                intervalo: Duration::from_secs(1),
            }),
            agente: None,
            redirecionamento: RedirecionamentoHttp::Seguir,
            aceitar_certificado_invalido: false,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        let mut tarefas = Vec::with_capacity(200);
        tarefas.extend(tarefas_concedidos());
        tarefas.extend(tarefas_emitidos());
        tarefas.extend(tarefas_mantidos());
        tarefas
    }
}

fn tarefas_concedidos() -> Vec<TarefaPlanejada> {
    CONCEDIDOS
        .iter()
        .map(|&(ym, url)| {
            let ano = &ym[..4];
            let mes = &ym[4..];
            TarefaPlanejada::nova(TarefaDeArquivo::nova(
                format!("concedidos_{ym}"),
                NOME,
                format!("concedidos/competencia={ym}"),
                url.to_owned(),
            ))
            .com_etiquetas([("registro", "concedidos"), ("ano", ano), ("mes", mes)])
        })
        .collect()
}

fn pasta_pda(ano: u16, mes: u8) -> &'static str {
    if ano < 2025 || (ano == 2025 && mes <= 6) {
        "PDA_2023_2025"
    } else {
        "PDA_2025_2027"
    }
}

fn iterar_meses(
    primeiro_ano: u16,
    primeiro_mes: u8,
    ultimo_ano: u16,
    ultimo_mes: u8,
) -> Vec<(u16, u8)> {
    let mut meses = Vec::new();
    let mut ano = primeiro_ano;
    let mut mes = primeiro_mes;
    loop {
        meses.push((ano, mes));
        if ano == ultimo_ano && mes == ultimo_mes {
            break;
        }
        mes += 1;
        if mes > 12 {
            mes = 1;
            ano += 1;
        }
    }
    meses
}

fn url_emitidos(pasta: &str, ym: &str, ano: u16, mes: u8) -> String {
    if ano >= 2026 && mes >= 7 {
        format!(
            "{BASE_S3}/{pasta}/Grupos_de_dados/Benef%C3%ADcios+emitidos/\
             D.DLK.FRM.000.DADOSABERTOS.EMITIDOS_{ym}.zip"
        )
    } else if ano >= 2026 && mes >= 4 {
        format!(
            "{BASE_S3}/{pasta}/Grupos_de_dados/Benef%C3%ADcios+emitidos/\
             D.SDA.PDA.003.EMI.{ym}.zip"
        )
    } else {
        format!(
            "{BASE_S3}/{pasta}/Grupos_de_dados/Benef%C3%ADcios+emitidos/\
             D.SDA.PDA.003.EMI.{ym}.CSV.ZIP"
        )
    }
}

fn tarefas_emitidos() -> Vec<TarefaPlanejada> {
    iterar_meses(PRIMEIRO_ANO, PRIMEIRO_MES, ULTIMO_ANO, ULTIMO_MES_EMITIDOS)
        .into_iter()
        .map(|(ano, mes)| {
            let ym = format!("{ano}{mes:02}");
            let ano_s = ano.to_string();
            let mes_s = format!("{mes:02}");
            let pasta = pasta_pda(ano, mes);
            let url = url_emitidos(pasta, &ym, ano, mes);
            let mut tarefa = TarefaDeArquivo::nova(
                format!("emitidos_{ym}"),
                NOME,
                format!("emitidos/competencia={ym}"),
                url,
            );
            if (ano == 2024 && mes == 4) || (ano == ULTIMO_ANO && mes >= ULTIMO_MES_EMITIDOS - 1) {
                tarefa = tarefa.aceitar_ausencia();
            }
            TarefaPlanejada::nova(tarefa).com_etiquetas([
                ("registro", "emitidos"),
                ("ano", &ano_s),
                ("mes", &mes_s),
            ])
        })
        .collect()
}

fn url_mantidos(pasta: &str, ym: &str, ano: u16, mes: u8, status: &str) -> String {
    if ano >= 2026 && mes >= 7 {
        let sufixo = match status {
            "ativos" => "ATIVOS",
            "cessados" => "CESSADOS",
            _ => "SUSPENSOS",
        };
        format!(
            "{BASE_S3}/{pasta}/Grupos_de_dados/Benef%C3%ADcios+mantidos/\
             D.DLK.FRM.000.DADOSABERTOS.MANTIDOS_{sufixo}_{ym}.zip"
        )
    } else if (ano == 2025 && mes >= 11) || (ano == 2026 && mes <= 6) {
        let curto = match status {
            "ativos" => "ATIVO",
            "cessados" => "CESSADO",
            _ => "SUSPENSO",
        };
        format!(
            "{BASE_S3}/{pasta}/Grupos_de_dados/Benef%C3%ADcios+mantidos/\
             {curto}_{ym}.zip"
        )
    } else {
        let (prefixo, ext) = match status {
            "ativos" => ("MANATIVOS", "CVS"),
            "cessados" => ("MANCESSADOS", "CSV"),
            _ => ("MANSUSPENSOS", "CSV"),
        };
        format!(
            "{BASE_S3}/{pasta}/Grupos_de_dados/Benef%C3%ADcios+mantidos/\
             D.SDA.PDA.004.{prefixo}.{ym}.{ext}.ZIP"
        )
    }
}

fn tarefas_mantidos() -> Vec<TarefaPlanejada> {
    let status_list: [&str; 3] = ["ativos", "cessados", "suspensos"];
    iterar_meses(PRIMEIRO_ANO, PRIMEIRO_MES, ULTIMO_ANO, ULTIMO_MES_MANTIDOS)
        .into_iter()
        .flat_map(|(ano, mes)| {
            let ym = format!("{ano}{mes:02}");
            let ano_s = ano.to_string();
            let mes_s = format!("{mes:02}");
            let pasta = pasta_pda(ano, mes);
            status_list.iter().map(move |&status| {
                let url = url_mantidos(pasta, &ym, ano, mes, status);
                let mut tarefa = TarefaDeArquivo::nova(
                    format!("mantidos_{status}_{ym}"),
                    NOME,
                    format!("mantidos/status={status}/competencia={ym}"),
                    url,
                );
                if (ano == 2024 && mes == 4)
                    || (ano == ULTIMO_ANO && mes >= ULTIMO_MES_MANTIDOS - 1)
                {
                    tarefa = tarefa.aceitar_ausencia();
                }
                TarefaPlanejada::nova(tarefa).com_etiquetas([
                    ("registro", "mantidos"),
                    ("status", status),
                    ("ano", &ano_s),
                    ("mes", &mes_s),
                ])
            })
        })
        .collect()
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::collections::HashSet;

    fn tarefas() -> Vec<TarefaPlanejada> {
        FontePrevidencia.criar_tarefas()
    }

    fn contagem(registro: &str) -> usize {
        tarefas()
            .iter()
            .filter(|t| t.etiquetas.get("registro").map(String::as_str) == Some(registro))
            .count()
    }

    #[test]
    fn gera_quantidade_esperada_de_tarefas() {
        let t = tarefas();
        let concedidos = CONCEDIDOS.len();
        let meses_emitidos =
            iterar_meses(PRIMEIRO_ANO, PRIMEIRO_MES, ULTIMO_ANO, ULTIMO_MES_EMITIDOS).len();
        let meses_mantidos =
            iterar_meses(PRIMEIRO_ANO, PRIMEIRO_MES, ULTIMO_ANO, ULTIMO_MES_MANTIDOS).len();
        let esperado = concedidos + meses_emitidos + meses_mantidos * 3;
        assert_eq!(
            t.len(),
            esperado,
            "concedidos={concedidos}, emitidos={meses_emitidos}, mantidos={}",
            meses_mantidos * 3
        );
    }

    #[test]
    fn nomes_sao_unicos() {
        let t = tarefas();
        let nomes: HashSet<_> = t.iter().map(|t| t.tarefa.nome()).collect();
        assert_eq!(nomes.len(), t.len());
    }

    #[test]
    fn concedidos_202306_existe() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "concedidos_202306")
        );
    }

    #[test]
    fn emitidos_202501_existe() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "emitidos_202501")
        );
    }

    #[test]
    fn mantidos_ativos_202412_existe() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "mantidos_ativos_202412")
        );
    }

    #[test]
    fn mantidos_tres_status_por_mes() {
        let t = tarefas();
        assert!(
            t.iter()
                .any(|t| t.tarefa.nome() == "mantidos_ativos_202306")
        );
        assert!(
            t.iter()
                .any(|t| t.tarefa.nome() == "mantidos_cessados_202306")
        );
        assert!(
            t.iter()
                .any(|t| t.tarefa.nome() == "mantidos_suspensos_202306")
        );
    }

    #[test]
    fn etiquetas_presentes() {
        for t in &tarefas() {
            let etiq = &t.etiquetas;
            assert!(
                etiq.contains_key("registro"),
                "{} sem etiqueta registro",
                t.tarefa.nome()
            );
            assert!(
                etiq.contains_key("ano"),
                "{} sem etiqueta ano",
                t.tarefa.nome()
            );
            assert!(
                etiq.contains_key("mes"),
                "{} sem etiqueta mes",
                t.tarefa.nome()
            );
        }
    }

    #[test]
    fn mantidos_tem_etiqueta_status() {
        for t in tarefas()
            .iter()
            .filter(|t| t.etiquetas.get("registro").map(String::as_str) == Some("mantidos"))
        {
            assert!(
                t.etiquetas.contains_key("status"),
                "{} sem etiqueta status",
                t.tarefa.nome()
            );
        }
    }

    #[test]
    fn e_sob_demanda() {
        assert!(FontePrevidencia.sob_demanda());
    }

    #[test]
    fn declara_limite_do_host() {
        let limite = FontePrevidencia
            .perfil_http()
            .limite
            .expect("limite do host S3");
        assert_eq!(
            limite.host,
            "armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com"
        );
    }

    #[test]
    fn contagem_fixa_por_registro() {
        assert_eq!(contagem("concedidos"), 41);
        assert_eq!(contagem("emitidos"), 40);
        assert_eq!(contagem("mantidos"), 120);
    }

    #[test]
    fn emitidos_jul2026_usa_padrao_dlk() {
        assert!(
            url_emitidos("PDA_2025_2027", "202607", 2026, 7)
                .contains("D.DLK.FRM.000.DADOSABERTOS.EMITIDOS_202607")
        );
    }

    #[test]
    fn emitidos_abr2026_usa_zip_minusculo() {
        let url = url_emitidos("PDA_2025_2027", "202604", 2026, 4);
        assert!(url.ends_with(".zip"));
        assert!(!url.contains(".CSV.ZIP"));
    }

    #[test]
    fn mantidos_jul2026_usa_padrao_dlk() {
        assert!(
            url_mantidos("PDA_2025_2027", "202607", 2026, 7, "ativos")
                .contains("D.DLK.FRM.000.DADOSABERTOS.MANTIDOS_ATIVOS_202607")
        );
    }

    #[test]
    fn mantidos_nov2025_usa_padrao_curto() {
        let url = url_mantidos("PDA_2025_2027", "202511", 2025, 11, "cessados");
        assert!(url.contains("CESSADO_202511.zip"));
    }
}
