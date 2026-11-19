use std::time::Duration;

use todeolho_nucleo::{
    FonteDeDados, LimiteHttp, PerfilHttp, RedirecionamentoHttp, TarefaPlanejada,
};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "datasus";

const FTP: &str = "ftp://ftp.datasus.gov.br/dissemin/publicos";

const UFS: [&str; 27] = [
    "AC", "AL", "AM", "AP", "BA", "CE", "DF", "ES", "GO", "MA", "MG", "MS", "MT", "PA", "PB", "PE",
    "PI", "PR", "RJ", "RN", "RO", "RR", "RS", "SC", "SE", "SP", "TO",
];

const PRIMEIRO_ANO_SIM: u16 = 1996;
const ULTIMO_ANO_SIM: u16 = 2023;

const PRIMEIRO_ANO_SINASC: u16 = 1996;
const ULTIMO_ANO_SINASC: u16 = 2023;

const PRIMEIRO_ANO_SIH: u16 = 2008;
const ULTIMO_ANO_SIH: u16 = 2024;

const PRIMEIRO_ANO_CNES: u16 = 2005;
const PRIMEIRO_MES_CNES: u8 = 8;
const ULTIMO_ANO_CNES: u16 = 2025;
const ULTIMO_MES_CNES: u8 = 1;

/// Indicadores de saúde do DATASUS — SIM, SINASC, SIH e CNES.
///
/// - **SIM** (1996–2023): declarações de óbito (DORES) e óbitos fetais
///   (DOFET), anuais por UF, formato DBC.
/// - **SINASC** (1996–2023): declarações de nascidos vivos (DN), anuais
///   por UF, formato DBC.
/// - **SIH** (2008-01 a 2024-12): internações hospitalares (RD), mensais
///   por UF, formato DBC.
/// - **CNES** (2005-08 a 2025-01): estabelecimentos de saúde (ST),
///   mensais por UF, formato DBC.
pub struct FonteDataSus;

impl FonteDeDados for FonteDataSus {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Indicadores de saúde do DATASUS — SIM, SINASC, SIH e CNES"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            credencial: None,
            limite: Some(LimiteHttp {
                host: "ftp.datasus.gov.br",
                requisicoes: 2,
                intervalo: Duration::from_secs(1),
            }),
            agente: None,
            redirecionamento: RedirecionamentoHttp::Seguir,
            aceitar_certificado_invalido: false,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        let mut tarefas = Vec::with_capacity(14100);
        tarefas.extend(tarefas_sim_dores());
        tarefas.extend(tarefas_sim_dofet());
        tarefas.extend(tarefas_sinasc());
        tarefas.extend(tarefas_sih());
        tarefas.extend(tarefas_cnes());
        tarefas
    }
}

fn tarefas_sim(subset: &str, dir: &str) -> Vec<TarefaPlanejada> {
    let mut tarefas = Vec::with_capacity(756);
    for ano in PRIMEIRO_ANO_SIM..=ULTIMO_ANO_SIM {
        let ano_s = ano.to_string();
        for &uf in &UFS {
            let uf_lower = uf.to_ascii_lowercase();
            let url = format!("{FTP}/SIM/CID10/{dir}/DO{uf}{ano}.dbc");
            let mut tarefa = TarefaDeArquivo::nova(
                format!("sim_{subset}_{uf_lower}_{ano}"),
                NOME,
                format!("sim/{subset}/ano={ano}/uf={uf_lower}"),
                url,
            );
            if ano == ULTIMO_ANO_SIM {
                tarefa = tarefa.aceitar_ausencia();
            }
            tarefas.push(TarefaPlanejada::nova(tarefa).com_etiquetas([
                ("registro", "sim"),
                ("subset", subset),
                ("ano", &ano_s),
                ("uf", &uf_lower),
            ]));
        }
    }
    tarefas
}

fn tarefas_sim_dores() -> Vec<TarefaPlanejada> {
    tarefas_sim("dores", "DORES")
}

fn tarefas_sim_dofet() -> Vec<TarefaPlanejada> {
    tarefas_sim("dofet", "DOFET")
}

fn tarefas_sinasc() -> Vec<TarefaPlanejada> {
    let mut tarefas = Vec::with_capacity(756);
    for ano in PRIMEIRO_ANO_SINASC..=ULTIMO_ANO_SINASC {
        let ano_s = ano.to_string();
        for &uf in &UFS {
            let uf_lower = uf.to_ascii_lowercase();
            let url = format!("{FTP}/SINASC/1996_/Dados/DNRES/DN{uf}{ano}.dbc");
            let mut tarefa = TarefaDeArquivo::nova(
                format!("sinasc_{uf_lower}_{ano}"),
                NOME,
                format!("sinasc/ano={ano}/uf={uf_lower}"),
                url,
            );
            if ano == ULTIMO_ANO_SINASC {
                tarefa = tarefa.aceitar_ausencia();
            }
            tarefas.push(TarefaPlanejada::nova(tarefa).com_etiquetas([
                ("registro", "sinasc"),
                ("ano", &ano_s),
                ("uf", &uf_lower),
            ]));
        }
    }
    tarefas
}

fn iterar_meses(
    primeiro_ano: u16,
    primeiro_mes: u8,
    ultimo_ano: u16,
    ultimo_mes: u8,
) -> impl Iterator<Item = (u16, u8)> {
    (primeiro_ano..=ultimo_ano).flat_map(move |ano| {
        let m_ini = if ano == primeiro_ano { primeiro_mes } else { 1 };
        let m_fim = if ano == ultimo_ano { ultimo_mes } else { 12 };
        (m_ini..=m_fim).map(move |mes| (ano, mes))
    })
}

fn tarefas_sih() -> Vec<TarefaPlanejada> {
    let mut tarefas = Vec::with_capacity(5510);
    for (ano, mes) in iterar_meses(PRIMEIRO_ANO_SIH, 1, ULTIMO_ANO_SIH, 12) {
        let ano_s = ano.to_string();
        let mes_s = format!("{mes:02}");
        let aa = format!("{:02}", ano % 100);
        let competencia = format!("{ano}{mes:02}");
        let ultimo_periodo = ano == ULTIMO_ANO_SIH && mes > 10;
        for &uf in &UFS {
            let uf_lower = uf.to_ascii_lowercase();
            let url = format!("{FTP}/SIHSUS/200801_/Dados/RD{uf}{aa}{mes_s}.dbc");
            let mut tarefa = TarefaDeArquivo::nova(
                format!("sih_{uf_lower}_{competencia}"),
                NOME,
                format!("sih/competencia={competencia}/uf={uf_lower}"),
                url,
            );
            if ultimo_periodo {
                tarefa = tarefa.aceitar_ausencia();
            }
            tarefas.push(TarefaPlanejada::nova(tarefa).com_etiquetas([
                ("registro", "sih"),
                ("ano", &ano_s),
                ("mes", &mes_s),
                ("uf", &uf_lower),
            ]));
        }
    }
    tarefas
}

fn tarefas_cnes() -> Vec<TarefaPlanejada> {
    let mut tarefas = Vec::with_capacity(6320);
    for (ano, mes) in iterar_meses(
        PRIMEIRO_ANO_CNES,
        PRIMEIRO_MES_CNES,
        ULTIMO_ANO_CNES,
        ULTIMO_MES_CNES,
    ) {
        let ano_s = ano.to_string();
        let mes_s = format!("{mes:02}");
        let aa = format!("{:02}", ano % 100);
        let competencia = format!("{ano}{mes:02}");
        let ultimo_periodo = (ano == ULTIMO_ANO_CNES && mes == ULTIMO_MES_CNES)
            || (ano == ULTIMO_ANO_CNES && mes + 1 == ULTIMO_MES_CNES);
        for &uf in &UFS {
            let uf_lower = uf.to_ascii_lowercase();
            let url = format!("{FTP}/CNES/200508_/Dados/ST/ST{uf}{aa}{mes_s}.dbc");
            let mut tarefa = TarefaDeArquivo::nova(
                format!("cnes_{uf_lower}_{competencia}"),
                NOME,
                format!("cnes/competencia={competencia}/uf={uf_lower}"),
                url,
            );
            if ultimo_periodo {
                tarefa = tarefa.aceitar_ausencia();
            }
            tarefas.push(TarefaPlanejada::nova(tarefa).com_etiquetas([
                ("registro", "cnes"),
                ("ano", &ano_s),
                ("mes", &mes_s),
                ("uf", &uf_lower),
            ]));
        }
    }
    tarefas
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::collections::HashSet;

    fn tarefas() -> Vec<TarefaPlanejada> {
        FonteDataSus.criar_tarefas()
    }

    fn contagem(registro: &str) -> usize {
        tarefas()
            .iter()
            .filter(|t| t.etiquetas.get("registro").map(String::as_str) == Some(registro))
            .count()
    }

    #[test]
    fn contagem_fixa_por_registro() {
        assert_eq!(contagem("sim"), 1512);
        assert_eq!(contagem("sinasc"), 756);
        assert_eq!(contagem("sih"), 5508);
        assert_eq!(contagem("cnes"), 6318);
    }

    #[test]
    fn total_14094_tarefas() {
        assert_eq!(tarefas().len(), 14094);
    }

    #[test]
    fn nomes_sao_unicos() {
        let t = tarefas();
        let nomes: HashSet<_> = t.iter().map(|t| t.tarefa.nome()).collect();
        assert_eq!(nomes.len(), t.len());
    }

    #[test]
    fn sim_dores_sp_2023_existe() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "sim_dores_sp_2023")
        );
    }

    #[test]
    fn sim_dofet_rj_2020_existe() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "sim_dofet_rj_2020")
        );
    }

    #[test]
    fn sinasc_mg_2022_existe() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "sinasc_mg_2022")
        );
    }

    #[test]
    fn sih_sp_202401_existe() {
        assert!(tarefas().iter().any(|t| t.tarefa.nome() == "sih_sp_202401"));
    }

    #[test]
    fn cnes_ba_200508_existe() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "cnes_ba_200508")
        );
    }

    #[test]
    fn sih_usa_ano_dois_digitos() {
        let url = format!("{FTP}/SIHSUS/200801_/Dados/RDSP2401.dbc");
        assert!(url.contains("RDSP2401"));
    }

    #[test]
    fn cnes_primeiro_mes_e_agosto_2005() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "cnes_ac_200508")
        );
        assert!(
            !tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "cnes_ac_200507")
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
                etiq.contains_key("uf"),
                "{} sem etiqueta uf",
                t.tarefa.nome()
            );
        }
    }

    #[test]
    fn e_sob_demanda() {
        assert!(FonteDataSus.sob_demanda());
    }

    #[test]
    fn declara_limite_do_host() {
        let limite = FonteDataSus
            .perfil_http()
            .limite
            .expect("limite do host DATASUS");
        assert_eq!(limite.host, "ftp.datasus.gov.br");
    }
}
