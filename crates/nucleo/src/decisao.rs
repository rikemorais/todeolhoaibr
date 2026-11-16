//! Pré-checagem idempotente: decide, antes de buscar, se um recurso precisa ser
//! (re)ingerido ou se o que já está na camada Raw serve (ver issue #13).
//!
//! Esta é a regra pura, sem I/O: recebe o que a origem informou (sem baixar,
//! via `ClienteHttp::informacoes`) e o estado do recurso na Raw, e devolve a
//! decisão. Ler o manifesto e verificar integridade fica a cargo de quem monta
//! o `EstadoNaRaw`.

use crate::portas::InformacoesDoArquivo;

/// Por que um recurso precisa ser (re)ingerido.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotivoDeIngestao {
    /// Não há ingestão completa anterior na Raw.
    AusenteNaRaw,
    /// A última ingestão não terminou (sem manifesto) ou alguma parte não
    /// confere com o manifesto (bytes ou sha256 divergentes).
    IncompletaOuCorrompida,
    /// O que está na Raw é menor do que a origem informa agora.
    Menor,
    /// A origem mudou desde a última ingestão (validador diferente, ou não há
    /// como confirmar que o que temos continua atual).
    Desatualizada,
}

/// Resultado da pré-checagem de um recurso.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisaoDeIngestao {
    /// A Raw já tem a mesma versão da origem; nada a baixar.
    Reaproveitar,
    /// Precisa (re)ingerir, com o motivo.
    Ingerir(MotivoDeIngestao),
}

/// O que já existe na Raw para um recurso, resumido para a decisão.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EstadoNaRaw {
    /// A última ingestão terminou (o manifesto está presente).
    pub completa: bool,
    /// As partes gravadas conferem com o manifesto (bytes e sha256).
    pub integra: bool,
    /// Validador da versão guardada (ETag/Last-Modified), quando houver.
    pub versao: Option<String>,
    /// Tamanho total guardado, em bytes, quando conhecido.
    pub tamanho: Option<u64>,
}

impl EstadoNaRaw {
    /// Estado em que a Raw tem algo registrado, mas não dá para reaproveitar:
    /// ponteiro ou manifesto ausente, ilegível ou sem partes. Força a reingestão
    /// com motivo `IncompletaOuCorrompida`, preservando o validador conhecido.
    pub fn incompleto(versao: Option<String>) -> Self {
        Self {
            completa: false,
            integra: false,
            versao,
            tamanho: None,
        }
    }
}

/// Decide se um recurso precisa ser (re)ingerido.
///
/// `raw` é `None` quando não há ingestão anterior. A ordem de precedência:
/// ausência, integridade, validador da origem e, por fim, tamanho.
///
/// # Exemplos
///
/// ```
/// use todeolho_nucleo::{DecisaoDeIngestao, EstadoNaRaw, MotivoDeIngestao, decidir};
/// use todeolho_nucleo::InformacoesDoArquivo;
///
/// // Sem nada na Raw, ingere pela primeira vez.
/// let origem = InformacoesDoArquivo { versao: Some("v1".into()), tamanho: Some(100) };
/// assert_eq!(
///     decidir(&origem, None),
///     DecisaoDeIngestao::Ingerir(MotivoDeIngestao::AusenteNaRaw),
/// );
///
/// // Validador igual ao guardado: reaproveita.
/// let raw = EstadoNaRaw { completa: true, integra: true, versao: Some("v1".into()), tamanho: Some(100) };
/// assert_eq!(decidir(&origem, Some(&raw)), DecisaoDeIngestao::Reaproveitar);
/// ```
pub fn decidir(origem: &InformacoesDoArquivo, raw: Option<&EstadoNaRaw>) -> DecisaoDeIngestao {
    use DecisaoDeIngestao::{Ingerir, Reaproveitar};
    use MotivoDeIngestao::{AusenteNaRaw, Desatualizada, IncompletaOuCorrompida, Menor};

    let Some(raw) = raw else {
        return Ingerir(AusenteNaRaw);
    };

    if !raw.completa || !raw.integra {
        return Ingerir(IncompletaOuCorrompida);
    }

    // Validador é autoritativo: se a origem informa um e ele bate com o nosso,
    // o conteúdo é o mesmo; se difere, mudou.
    if let Some(versao_origem) = &origem.versao {
        return match &raw.versao {
            Some(versao_raw) if versao_raw == versao_origem => Reaproveitar,
            _ => Ingerir(Desatualizada),
        };
    }

    // Sem validador na origem: cai para o tamanho.
    match (origem.tamanho, raw.tamanho) {
        (Some(na_origem), Some(na_raw)) if na_raw < na_origem => Ingerir(Menor),
        (Some(na_origem), Some(na_raw)) if na_raw == na_origem => Reaproveitar,
        // Tamanho maior que a origem, ou desconhecido dos dois lados: não dá
        // para confirmar que o que temos continua atual, então reingere.
        _ => Ingerir(Desatualizada),
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn origem(versao: Option<&str>, tamanho: Option<u64>) -> InformacoesDoArquivo {
        InformacoesDoArquivo {
            versao: versao.map(str::to_owned),
            tamanho,
        }
    }

    fn na_raw(versao: Option<&str>, tamanho: Option<u64>) -> EstadoNaRaw {
        EstadoNaRaw {
            completa: true,
            integra: true,
            versao: versao.map(str::to_owned),
            tamanho,
        }
    }

    #[test]
    fn ingere_quando_ausente_na_raw() {
        let decisao = decidir(&origem(Some("v1"), Some(100)), None);
        assert_eq!(
            decisao,
            DecisaoDeIngestao::Ingerir(MotivoDeIngestao::AusenteNaRaw)
        );
    }

    #[test]
    fn reingere_quando_incompleta() {
        let raw = EstadoNaRaw {
            completa: false,
            ..na_raw(Some("v1"), Some(100))
        };
        let decisao = decidir(&origem(Some("v1"), Some(100)), Some(&raw));
        assert_eq!(
            decisao,
            DecisaoDeIngestao::Ingerir(MotivoDeIngestao::IncompletaOuCorrompida)
        );
    }

    #[test]
    fn reingere_quando_corrompida() {
        let raw = EstadoNaRaw {
            integra: false,
            ..na_raw(Some("v1"), Some(100))
        };
        let decisao = decidir(&origem(Some("v1"), Some(100)), Some(&raw));
        assert_eq!(
            decisao,
            DecisaoDeIngestao::Ingerir(MotivoDeIngestao::IncompletaOuCorrompida)
        );
    }

    #[test]
    fn reaproveita_quando_o_validador_bate() {
        let decisao = decidir(
            &origem(Some("v1"), Some(100)),
            Some(&na_raw(Some("v1"), Some(100))),
        );
        assert_eq!(decisao, DecisaoDeIngestao::Reaproveitar);
    }

    #[test]
    fn reingere_quando_o_validador_difere() {
        let decisao = decidir(
            &origem(Some("v2"), Some(100)),
            Some(&na_raw(Some("v1"), Some(100))),
        );
        assert_eq!(
            decisao,
            DecisaoDeIngestao::Ingerir(MotivoDeIngestao::Desatualizada)
        );
    }

    #[test]
    fn validador_bate_ignora_tamanho_diferente() {
        // Se a origem confirma a mesma versão, o tamanho não muda a decisão.
        let decisao = decidir(
            &origem(Some("v1"), Some(200)),
            Some(&na_raw(Some("v1"), Some(100))),
        );
        assert_eq!(decisao, DecisaoDeIngestao::Reaproveitar);
    }

    #[test]
    fn sem_validador_reingere_quando_menor() {
        let decisao = decidir(&origem(None, Some(100)), Some(&na_raw(None, Some(50))));
        assert_eq!(decisao, DecisaoDeIngestao::Ingerir(MotivoDeIngestao::Menor));
    }

    #[test]
    fn sem_validador_reaproveita_quando_o_tamanho_bate() {
        let decisao = decidir(&origem(None, Some(100)), Some(&na_raw(None, Some(100))));
        assert_eq!(decisao, DecisaoDeIngestao::Reaproveitar);
    }

    #[test]
    fn sem_validador_e_sem_tamanho_reingere() {
        let decisao = decidir(&origem(None, None), Some(&na_raw(None, None)));
        assert_eq!(
            decisao,
            DecisaoDeIngestao::Ingerir(MotivoDeIngestao::Desatualizada)
        );
    }

    #[test]
    fn origem_com_validador_e_raw_sem_reingere() {
        let decisao = decidir(
            &origem(Some("v1"), Some(100)),
            Some(&na_raw(None, Some(100))),
        );
        assert_eq!(
            decisao,
            DecisaoDeIngestao::Ingerir(MotivoDeIngestao::Desatualizada)
        );
    }
}
