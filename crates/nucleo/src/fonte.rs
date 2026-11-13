//! O contrato [`FonteDeDados`]: cada API pública ingerida implementa este trait,
//! que decide quais tarefas existem; montar e executar o grafo é igual para todas.

use crate::dag::DagDeIngestao;
use crate::erros::ErroDeDag;
use crate::tarefa::TarefaPlanejada;

/// Credencial HTTP declarada sem expor o valor do segredo ao domínio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CredencialHttp {
    /// Host autorizado a receber a credencial, sem esquema ou porta.
    pub host: &'static str,
    /// Variável de ambiente que contém o segredo.
    pub variavel: &'static str,
    /// Nome do cabeçalho enviado à API.
    pub cabecalho: &'static str,
    /// Prefixo antes do segredo, por exemplo `Bearer `; vazio para chave direta.
    pub prefixo: &'static str,
}

/// Máximo de requisições permitidas para um host em um intervalo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LimiteHttp {
    /// Host ao qual o limite se aplica, sem esquema ou porta.
    pub host: &'static str,
    /// Número máximo de requisições no intervalo.
    pub requisicoes: u32,
    /// Duração do intervalo.
    pub intervalo: std::time::Duration,
}

/// Requisitos HTTP de uma fonte. O padrão é acesso anônimo sem limite.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PerfilHttp {
    /// Credencial opcional da fonte.
    pub credencial: Option<CredencialHttp>,
    /// Limite opcional da API.
    pub limite: Option<LimiteHttp>,
    /// User-Agent alternativo; quando `Some`, sobrescreve o padrão do projeto.
    pub agente: Option<&'static str>,
}

/// Uma API pública ingerida pelo projeto (o "criador" do Factory Method).
///
/// Cada fonte concreta só decide *quais* tarefas existem, em [`criar_tarefas`].
/// Validar o grafo e executá-lo é igual para todas, então novas fontes entram sem
/// alterar o executor. Ver `docs/adr/0004`.
///
/// [`criar_tarefas`]: FonteDeDados::criar_tarefas
pub trait FonteDeDados: Send + Sync {
    /// Identificador estável: é usado na CLI e no caminho dos dados na camada Raw.
    fn nome(&self) -> &str;

    /// Texto curto que descreve a fonte, exibido na CLI e no menu de seleção.
    fn descricao(&self) -> &str;

    /// Fontes muito grandes só rodam quando pedidas pelo nome, nunca em `--todas`.
    fn sob_demanda(&self) -> bool {
        false
    }

    /// Declara autenticação e limite de taxa sem carregar o segredo no domínio.
    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp::default()
    }

    /// Factory Method: cria as tarefas concretas desta fonte.
    fn criar_tarefas(&self) -> Vec<TarefaPlanejada>;

    /// Operação comum do criador: monta a DAG validada a partir do Factory Method.
    fn montar_dag(&self) -> Result<DagDeIngestao, ErroDeDag> {
        DagDeIngestao::construir(self.nome(), self.criar_tarefas())
    }
}
