use std::time::Duration;

use todeolho_nucleo::{
    FonteDeDados, LimiteHttp, PerfilHttp, RedirecionamentoHttp, TarefaPlanejada,
};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "trabalho";
const BASE: &str = "ftp://ftp.mtps.gov.br/pdet/microdados";

const PRIMEIRO_ANO_RAIS: u16 = 1985;
const ULTIMO_ANO_RAIS: u16 = 2025;
/// A RAIS mudou de per-UF para per-região em 2018.
const ANO_CORTE_RAIS: u16 = 2018;

const PRIMEIRO_ANO_CAGED: u16 = 2020;
const ULTIMO_ANO_CAGED: u16 = 2026;
/// Último mês publicado no FTP — atualizar quando o MTE liberar novos meses.
const ULTIMO_MES_CAGED: u8 = 9;

const UFS: [&str; 27] = [
    "AC", "AL", "AM", "AP", "BA", "CE", "DF", "ES", "GO", "MA", "MG", "MS", "MT", "PA", "PB", "PE",
    "PI", "PR", "RJ", "RN", "RO", "RR", "RS", "SC", "SE", "SP", "TO",
];

const REGIOES: [&str; 6] = ["CENTRO_OESTE", "MG_ES_RJ", "NORDESTE", "NORTE", "SP", "SUL"];

/// Emprego formal: RAIS (anual) e Novo CAGED (mensal) do PDET/MTE.
///
/// Microdados de vínculos (RAIS) e movimentações (Novo CAGED) disponíveis
/// via FTP anônimo em `ftp://ftp.mtps.gov.br/pdet/microdados/`.
///
/// A RAIS mudou o formato dos arquivos em 2018: de per-UF (27 arquivos)
/// para per-região (6 arquivos).
///
/// O Novo CAGED publica três tipos: MOV (desde 2020-01), FOR (desde
/// 2020-02) e EXC (desde 2020-04).
pub struct FonteTrabalho;

impl FonteDeDados for FonteTrabalho {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Emprego formal (RAIS e Novo CAGED) — microdados PDET/MTE"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            credencial: None,
            limite: Some(LimiteHttp {
                host: "ftp.mtps.gov.br",
                requisicoes: 2,
                intervalo: Duration::from_secs(1),
            }),
            agente: None,
            redirecionamento: RedirecionamentoHttp::Seguir,
            aceitar_certificado_invalido: false,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        let mut tarefas = Vec::with_capacity(1300);
        tarefas.extend(tarefas_rais_per_uf());
        tarefas.extend(tarefas_rais_per_regiao());
        tarefas.extend(tarefas_caged());
        tarefas
    }
}

fn tarefas_rais_per_uf() -> Vec<TarefaPlanejada> {
    let mut tarefas = Vec::new();
    for ano in PRIMEIRO_ANO_RAIS..ANO_CORTE_RAIS {
        let ano_s = ano.to_string();
        for uf in UFS {
            if (uf == "TO" && ano < 1990)
                || (uf == "MA" && ano == 1985)
                || (uf == "PA" && ano == 1986)
            {
                continue;
            }
            let uf_lower = uf.to_ascii_lowercase();
            let nome = format!("rais_{ano}_{uf_lower}");
            let recurso = format!("rais/tipo=vinculo/ano={ano}/uf={uf_lower}");
            let url = format!("{BASE}/RAIS/{ano}/{uf}{ano}.7z");
            let tarefa = TarefaDeArquivo::nova(nome, NOME, recurso, url);
            tarefas.push(TarefaPlanejada::nova(tarefa).com_etiquetas([
                ("registro", "rais"),
                ("tipo", "vinculo"),
                ("ano", &ano_s),
                ("uf", &uf_lower),
            ]));
        }
        // ESTB só existe a partir de 2007.
        if ano >= 2007 {
            let nome_estb = format!("rais_{ano}_estb");
            let recurso_estb = format!("rais/tipo=estabelecimento/ano={ano}");
            let url_estb = format!("{BASE}/RAIS/{ano}/ESTB{ano}.7z");
            let tarefa_estb = TarefaDeArquivo::nova(nome_estb, NOME, recurso_estb, url_estb);
            tarefas.push(TarefaPlanejada::nova(tarefa_estb).com_etiquetas([
                ("registro", "rais"),
                ("tipo", "estabelecimento"),
                ("ano", &ano_s),
            ]));
        }
    }
    tarefas
}

fn tarefas_rais_per_regiao() -> Vec<TarefaPlanejada> {
    let mut tarefas = Vec::new();
    for ano in ANO_CORTE_RAIS..=ULTIMO_ANO_RAIS {
        let ano_s = ano.to_string();
        let aceitar = ano == ULTIMO_ANO_RAIS;
        for regiao in REGIOES {
            let regiao_lower = regiao.to_ascii_lowercase();
            let nome = format!("rais_{ano}_{regiao_lower}");
            let recurso = format!("rais/tipo=vinculo/ano={ano}/regiao={regiao_lower}");
            let url = format!("{BASE}/RAIS/{ano}/RAIS_VINC_PUB_{regiao}.7z");
            let mut tarefa = TarefaDeArquivo::nova(nome, NOME, recurso, url);
            if aceitar {
                tarefa = tarefa.aceitar_ausencia();
            }
            tarefas.push(TarefaPlanejada::nova(tarefa).com_etiquetas([
                ("registro", "rais"),
                ("tipo", "vinculo"),
                ("ano", &ano_s),
                ("regiao", &regiao_lower),
            ]));
        }
        // NI (não identificados) só existe a partir de 2022.
        if ano >= 2022 {
            let nome_ni = format!("rais_{ano}_ni");
            let recurso_ni = format!("rais/tipo=vinculo/ano={ano}/regiao=ni");
            let url_ni = format!("{BASE}/RAIS/{ano}/RAIS_VINC_PUB_NI.7z");
            tarefas.push(
                TarefaPlanejada::nova(
                    TarefaDeArquivo::nova(nome_ni, NOME, recurso_ni, url_ni).aceitar_ausencia(),
                )
                .com_etiquetas([
                    ("registro", "rais"),
                    ("tipo", "vinculo"),
                    ("ano", &ano_s),
                    ("regiao", "ni"),
                ]),
            );
        }
        {
            let nome_estb = format!("rais_{ano}_estb");
            let recurso_estb = format!("rais/tipo=estabelecimento/ano={ano}");
            let url_estb = format!("{BASE}/RAIS/{ano}/RAIS_ESTAB_PUB.7z");
            let mut tarefa = TarefaDeArquivo::nova(nome_estb, NOME, recurso_estb, url_estb);
            if aceitar {
                tarefa = tarefa.aceitar_ausencia();
            }
            tarefas.push(TarefaPlanejada::nova(tarefa).com_etiquetas([
                ("registro", "rais"),
                ("tipo", "estabelecimento"),
                ("ano", &ano_s),
            ]));
        }
    }
    tarefas
}

fn tarefas_caged() -> Vec<TarefaPlanejada> {
    let mut tarefas = Vec::new();
    let tipos = [("MOV", "mov"), ("FOR", "for"), ("EXC", "exc")];
    for ano in PRIMEIRO_ANO_CAGED..=ULTIMO_ANO_CAGED {
        let ultimo_mes = if ano == ULTIMO_ANO_CAGED {
            ULTIMO_MES_CAGED
        } else {
            12
        };
        let ano_s = ano.to_string();
        for mes in 1..=ultimo_mes {
            let ym = format!("{ano}{mes:02}");
            let mes_s = format!("{mes:02}");
            for &(tipo_upper, tipo_lower) in &tipos {
                // EXC começa em 2020-04, FOR começa em 2020-02.
                if ano == 2020
                    && ((tipo_lower == "exc" && mes < 4) || (tipo_lower == "for" && mes < 2))
                {
                    continue;
                }
                let nome = format!("caged_{tipo_lower}_{ym}");
                let recurso = format!("caged/tipo={tipo_lower}/ano={ano}/mes={mes_s}");
                let url = format!("{BASE}/NOVO%20CAGED/{ano}/{ym}/CAGED{tipo_upper}{ym}.7z");
                let tarefa = TarefaDeArquivo::nova(nome, NOME, recurso, url);
                tarefas.push(TarefaPlanejada::nova(tarefa).com_etiquetas([
                    ("registro", "caged"),
                    ("tipo", tipo_lower),
                    ("ano", &ano_s),
                    ("mes", &mes_s),
                ]));
            }
        }
    }
    tarefas
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::collections::HashSet;

    fn tarefas() -> Vec<TarefaPlanejada> {
        FonteTrabalho.criar_tarefas()
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
        let anos_uf = (ANO_CORTE_RAIS - PRIMEIRO_ANO_RAIS) as usize;
        let uf_omitidas = 5 + 1 + 1; // TO < 1990 (5), MA 1985 (1), PA 1986 (1)
        let estb_uf = anos_uf - 22; // ESTB só desde 2007 (2007..2017 = 11)
        let rais_per_uf = anos_uf * UFS.len() - uf_omitidas + estb_uf;
        let anos_regiao = (ULTIMO_ANO_RAIS - ANO_CORTE_RAIS + 1) as usize;
        let anos_ni = (ULTIMO_ANO_RAIS - 2022 + 1) as usize;
        let rais_per_regiao = anos_regiao * (REGIOES.len() + 1) + anos_ni;
        let caged_meses =
            (ULTIMO_ANO_CAGED - PRIMEIRO_ANO_CAGED) as usize * 12 + ULTIMO_MES_CAGED as usize;
        let caged_omitidas = 3 + 1; // EXC 2020-01..03, FOR 2020-01
        let caged_total = caged_meses * 3 - caged_omitidas;
        let esperado = rais_per_uf + rais_per_regiao + caged_total;
        assert_eq!(
            t.len(),
            esperado,
            "RAIS UF={rais_per_uf}, RAIS reg={rais_per_regiao}, CAGED={caged_total}"
        );
    }

    #[test]
    fn nomes_sao_unicos() {
        let t = tarefas();
        let nomes: HashSet<_> = t.iter().map(|t| t.tarefa.nome()).collect();
        assert_eq!(nomes.len(), t.len());
    }

    #[test]
    fn rais_1985_ac_existe() {
        assert!(tarefas().iter().any(|t| t.tarefa.nome() == "rais_1985_ac"));
    }

    #[test]
    fn rais_2024_sp_existe() {
        assert!(tarefas().iter().any(|t| t.tarefa.nome() == "rais_2024_sp"));
    }

    #[test]
    fn rais_2024_estb_existe() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "rais_2024_estb")
        );
    }

    #[test]
    fn caged_mov_202001_existe() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "caged_mov_202001")
        );
    }

    #[test]
    fn caged_tres_tipos_por_mes() {
        let t = tarefas();
        assert!(t.iter().any(|t| t.tarefa.nome() == "caged_mov_202106"));
        assert!(t.iter().any(|t| t.tarefa.nome() == "caged_for_202106"));
        assert!(t.iter().any(|t| t.tarefa.nome() == "caged_exc_202106"));
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
                etiq.contains_key("tipo"),
                "{} sem etiqueta tipo",
                t.tarefa.nome()
            );
            assert!(
                etiq.contains_key("ano"),
                "{} sem etiqueta ano",
                t.tarefa.nome()
            );
        }
    }

    #[test]
    fn rais_tem_etiqueta_uf_ou_regiao() {
        for t in tarefas()
            .iter()
            .filter(|t| t.etiquetas.get("registro").map(String::as_str) == Some("rais"))
            .filter(|t| t.etiquetas.get("tipo").map(String::as_str) == Some("vinculo"))
        {
            let etiq = &t.etiquetas;
            assert!(
                etiq.contains_key("uf") || etiq.contains_key("regiao"),
                "{} sem uf nem regiao",
                t.tarefa.nome()
            );
        }
    }

    #[test]
    fn caged_tem_etiqueta_mes() {
        for t in tarefas()
            .iter()
            .filter(|t| t.etiquetas.get("registro").map(String::as_str) == Some("caged"))
        {
            assert!(
                t.etiquetas.contains_key("mes"),
                "{} sem etiqueta mes",
                t.tarefa.nome()
            );
        }
    }

    #[test]
    fn e_sob_demanda() {
        assert!(FonteTrabalho.sob_demanda());
    }

    #[test]
    fn declara_limite_do_host() {
        let limite = FonteTrabalho
            .perfil_http()
            .limite
            .expect("limite do host FTP");
        assert_eq!(limite.host, "ftp.mtps.gov.br");
    }

    #[test]
    fn contagem_rais_e_caged() {
        let anos_uf = (ANO_CORTE_RAIS - PRIMEIRO_ANO_RAIS) as usize;
        let uf_omitidas = 5 + 1 + 1;
        let estb_uf = anos_uf - 22;
        let rais_per_uf = anos_uf * UFS.len() - uf_omitidas + estb_uf;
        let anos_regiao = (ULTIMO_ANO_RAIS - ANO_CORTE_RAIS + 1) as usize;
        let anos_ni = (ULTIMO_ANO_RAIS - 2022 + 1) as usize;
        let rais_per_regiao = anos_regiao * (REGIOES.len() + 1) + anos_ni;
        assert_eq!(contagem("rais"), rais_per_uf + rais_per_regiao);
        assert!(contagem("caged") > 200);
    }
}
