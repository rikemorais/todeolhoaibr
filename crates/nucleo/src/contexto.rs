use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::portas::{ArmazenamentoRaw, ClienteHttp};

/// Dependências e identidade de uma execução, entregues a cada tarefa.
#[derive(Clone)]
pub struct ContextoDeExecucao {
    pub id_execucao: String,
    pub iniciada_em: DateTime<Utc>,
    pub armazenamento: Arc<dyn ArmazenamentoRaw>,
    pub http: Arc<dyn ClienteHttp>,
}
