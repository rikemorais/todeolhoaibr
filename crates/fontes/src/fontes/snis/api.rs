use std::time::Duration;

use todeolho_nucleo::{LimiteHttp, PerfilHttp, RedirecionamentoHttp};

pub(super) const FONTE_RAW: &str = "snis";

pub(super) const fn perfil_http() -> PerfilHttp {
    PerfilHttp {
        redirecionamento: RedirecionamentoHttp::Seguir,
        credencial: None,
        limite: Some(LimiteHttp {
            host: "www.gov.br",
            requisicoes: 5,
            intervalo: Duration::from_secs(1),
        }),
        agente: None,
        aceitar_certificado_invalido: false,
    }
}

pub(super) const AE_2023: &str = "https://www.gov.br/cidades/pt-br/acesso-a-informacao/acoes-e-programas/saneamento/sinisa/arquivos/SINISA_Resultados_Ref2023.zip";
pub(super) const AGUA_2024: &str = "https://www.gov.br/cidades/pt-br/acesso-a-informacao/acoes-e-programas/saneamento/sinisa/resultados-sinisa/SINISA_Resultados_Ref2024.zip";
pub(super) const ESGOTO_2024: &str = "https://www.gov.br/cidades/pt-br/acesso-a-informacao/acoes-e-programas/saneamento/sinisa/resultados-sinisa/SINISA_ESGOTO_Planilhas_2024.zip";
pub(super) const RESIDUOS_2024: &str = "https://www.gov.br/cidades/pt-br/acesso-a-informacao/acoes-e-programas/saneamento/sinisa/resultados-sinisa/SINISA_RESIDUOS_planilhas_2024.zip";
pub(super) const PLUVIAIS_2024: &str = "https://www.gov.br/cidades/pt-br/acesso-a-informacao/acoes-e-programas/saneamento/sinisa/resultados-sinisa/SINISA_AGUASPLUVIAIS_Informacoes_Indicadores_2025.zip";
