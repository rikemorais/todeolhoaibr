//! O contexto compartilhado por todas as tarefas de uma execução: identidade e
//! as portas de saída que elas usam para falar com o mundo externo.

use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::portas::{ArmazenamentoRaw, ClienteHttp};

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
}
