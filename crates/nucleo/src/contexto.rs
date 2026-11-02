//! O contexto compartilhado por todas as tarefas de uma execução: identidade e
//! as portas de saída que elas usam para falar com o mundo externo.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Utc};

use crate::portas::{ArmazenamentoRaw, ClienteHttp};

/// Progresso parcial de uma tarefa enquanto ela baixa, para o painel mostrar o
/// percentual em tempo real.
///
/// A tarefa só escreve (contadores atômicos, baratos e sem trava); quem observa
/// (o registrador) lê. `total = 0` significa tamanho ainda desconhecido.
#[derive(Debug, Default)]
pub struct ProgressoDaTarefa {
    baixados: AtomicU64,
    total: AtomicU64,
}

impl ProgressoDaTarefa {
    /// Anuncia o tamanho total esperado, quando a origem o informa.
    pub fn definir_total(&self, total: u64) {
        self.total.store(total, Ordering::Relaxed);
    }

    /// Soma os bytes recém-baixados ao acumulado.
    pub fn avancar(&self, bytes: u64) {
        self.baixados.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Bytes baixados até agora.
    pub fn baixados(&self) -> u64 {
        self.baixados.load(Ordering::Relaxed)
    }

    /// Tamanho total esperado, ou `None` enquanto desconhecido.
    pub fn total(&self) -> Option<u64> {
        match self.total.load(Ordering::Relaxed) {
            0 => None,
            total => Some(total),
        }
    }
}

/// Dependências e identidade de uma execução, entregues a cada tarefa.
#[derive(Clone)]
pub struct ContextoDeExecucao {
    /// Identifica esta execução; compõe o caminho das partes na camada Raw.
    pub id_execucao: String,
    /// Instante de início, usado no particionamento por data (`data_ingestao=`).
    pub iniciada_em: DateTime<Utc>,
    /// Porta de saída para gravar e ler os dados brutos.
    pub armazenamento: Arc<dyn ArmazenamentoRaw>,
    /// Porta de saída para acessar as APIs públicas.
    pub http: Arc<dyn ClienteHttp>,
    /// Para onde a tarefa reporta o quanto já baixou. O executor injeta um por
    /// tarefa; `None` quando ninguém acompanha (testes, estimativas).
    pub progresso: Option<Arc<ProgressoDaTarefa>>,
}
