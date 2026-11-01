//! Quanto uma ingestão vai ocupar e se isso cabe no disco (ver `docs/adr/0010`).

use std::sync::Arc;

use futures::{StreamExt, stream};

use crate::contexto::ContextoDeExecucao;
use crate::tarefa::Tarefa;

const CONSULTAS_SIMULTANEAS: usize = 8;

/// Tamanho previsto de cada tarefa, na mesma ordem. `None` significa desconhecido.
pub async fn estimar(
    tarefas: &[Arc<dyn Tarefa>],
    contexto: &ContextoDeExecucao,
) -> Vec<Option<u64>> {
    stream::iter(tarefas)
        .map(|tarefa| async move {
            // Falhar na estimativa não impede a escolha: o tamanho só fica desconhecido.
            tarefa.bytes_a_baixar(contexto).await.ok().flatten()
        })
        .buffered(CONSULTAS_SIMULTANEAS)
        .collect()
        .await
}

/// Resumo do espaço necessário para um conjunto de tarefas.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EspacoNecessario {
    /// Soma das estimativas conhecidas, em bytes.
    pub bytes: u64,
    /// Maior estimativa individual, usada para o pico durante o envio em partes.
    pub maior_arquivo: u64,
    /// Quantas tarefas não tiveram estimativa (tamanho desconhecido).
    pub sem_estimativa: usize,
}

impl EspacoNecessario {
    /// Agrega as estimativas de várias tarefas em um só resumo.
    pub fn somar<'a>(estimativas: impl IntoIterator<Item = &'a Option<u64>>) -> Self {
        let mut espaco = Self::default();
        for estimativa in estimativas {
            match estimativa {
                Some(bytes) => {
                    espaco.bytes += bytes;
                    espaco.maior_arquivo = espaco.maior_arquivo.max(*bytes);
                }
                None => espaco.sem_estimativa += 1,
            }
        }
        espaco
    }

    /// Durante um envio em partes, o armazenamento guarda as partes e o arquivo final ao
    /// mesmo tempo. Por isso o pico inclui o maior arquivo uma segunda vez.
    pub fn pico(&self) -> u64 {
        self.bytes + self.maior_arquivo
    }

    /// Compara o pico necessário com o espaço livre informado.
    pub fn avaliar(&self, livre: Option<u64>) -> AvaliacaoDeEspaco {
        match livre {
            None => AvaliacaoDeEspaco::Desconhecido,
            Some(livre) if self.pico() > livre => AvaliacaoDeEspaco::NaoCabe {
                faltam: self.pico() - livre,
            },
            Some(_) => AvaliacaoDeEspaco::Cabe,
        }
    }
}

/// Veredito de [`EspacoNecessario::avaliar`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvaliacaoDeEspaco {
    /// O pico necessário cabe no espaço livre.
    Cabe,
    /// Não cabe; faltam `faltam` bytes.
    NaoCabe {
        /// Quantos bytes faltam para o pico caber.
        faltam: u64,
    },
    /// O espaço livre não pôde ser medido.
    Desconhecido,
}

/// Formata bytes para leitura, no padrão brasileiro: `1,7 GB`.
///
/// # Exemplos
///
/// ```
/// use todeolho_nucleo::formatar_bytes;
///
/// assert_eq!(formatar_bytes(194), "194 B");
/// assert_eq!(formatar_bytes(2_470_036), "2,4 MB");
/// assert_eq!(formatar_bytes(1_800_000_000), "1,7 GB");
/// ```
pub fn formatar_bytes(bytes: u64) -> String {
    const UNIDADES: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut valor = bytes as f64;
    let mut indice = 0;
    while valor >= 1024.0 && indice < UNIDADES.len() - 1 {
        valor /= 1024.0;
        indice += 1;
    }
    if indice == 0 {
        format!("{bytes} B")
    } else {
        format!("{valor:.1} {}", UNIDADES[indice]).replace('.', ",")
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn soma_e_conta_o_que_nao_tem_estimativa() {
        let espaco = EspacoNecessario::somar(&[Some(10), None, Some(30)]);

        assert_eq!(espaco.bytes, 40);
        assert_eq!(espaco.maior_arquivo, 30);
        assert_eq!(espaco.sem_estimativa, 1);
    }

    #[test]
    fn o_pico_inclui_o_maior_arquivo_duas_vezes() {
        let espaco = EspacoNecessario::somar(&[Some(100), Some(30)]);

        assert_eq!(espaco.pico(), 230);
        assert_eq!(espaco.avaliar(Some(230)), AvaliacaoDeEspaco::Cabe);
        assert_eq!(
            espaco.avaliar(Some(229)),
            AvaliacaoDeEspaco::NaoCabe { faltam: 1 }
        );
        // O total sozinho caberia, mas não haveria espaço para o pico.
        assert_eq!(
            espaco.avaliar(Some(130)),
            AvaliacaoDeEspaco::NaoCabe { faltam: 100 }
        );
    }

    #[test]
    fn espaco_livre_desconhecido_nao_e_tratado_como_suficiente() {
        assert_eq!(
            EspacoNecessario::default().avaliar(None),
            AvaliacaoDeEspaco::Desconhecido
        );
    }

    #[test]
    fn formata_no_padrao_brasileiro() {
        assert_eq!(formatar_bytes(194), "194 B");
        assert_eq!(formatar_bytes(2_470_036), "2,4 MB");
        assert_eq!(formatar_bytes(1_800_000_000), "1,7 GB");
    }
}
