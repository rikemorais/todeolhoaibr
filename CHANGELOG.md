# Histórico de mudanças

Todas as mudanças relevantes do projeto ficam registradas aqui.

O formato segue o [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/), e o projeto segue o
[Versionamento Semântico](https://semver.org/lang/pt-BR/). Veja o [ADR 0011](docs/adr/0011-versionamento-e-commits.md).

## [Não lançado]

### Adicionado

- Bronze para fontes em **JSON de API**, via um `DecodificadorJson` genérico (array de registros no topo ou sob uma chave; campos viram colunas de texto, aninhados viram JSON compacto): `bcb`, `ipeadata`, `camara-tramitacoes`, `camara-deputados`, `camara-partidos`, `camara-legislaturas`, `ibge-estados`, `ibge-regioes`, `ibge-municipios`, `senado-processos` e `senado-votacoes` ([issue #168](https://github.com/rikemorais/todeolhoaibr/issues/168)).
- Bronze para os **agregados do IBGE** (API v3, JSON aninhado): `ibge-ipca`, `ibge-inpc`, `ibge-pib-municipios`, `ibge-pib-trimestral`, `ibge-pnad-desocupacao` e `ibge-populacao-municipios`. Um `DecodificadorIbgeAgregados` explode a série aninhada em uma linha por (variável, localidade, período) ([issue #168](https://github.com/rikemorais/todeolhoaibr/issues/168)).
- Bronze para `senado-parlamentares` (array aninhado fundo no JSON do Senado); o `DecodificadorJson` passou a aceitar um **caminho** de chaves até o array ([issue #168](https://github.com/rikemorais/todeolhoaibr/issues/168)).
- Bronze para `senado-processos-detalhes` (um JSON por processo, ~163k arquivos): um decodificador **agrega** todos os processos de um ano em uma tabela (uma linha por processo), em vez de gerar um Parquet por arquivo. A `TarefaDeBronze` passou a decodificar as partes de um recurso em bloco, permitindo essa agregação ([issue #177](https://github.com/rikemorais/todeolhoaibr/issues/177)).
- O decodificador ZIP passou a aceitar também arquivos `.txt` (os anos antigos do TSE usam `.txt` no lugar de `.csv`, mesmo formato `;`/Latin-1) ([issue #168](https://github.com/rikemorais/todeolhoaibr/issues/168)).
- **Refino em fluxo de ponta a ponta** para arquivos grandes ([issue #171](https://github.com/rikemorais/todeolhoaibr/issues/171)): o part do raw é baixado do S3 **direto para um arquivo temporário** (novo `ArmazenamentoRaw::ler_fluxo`), o ZIP é lido do disco, cada CSV é decodificado em fluxo (Latin-1→UTF-8 on the fly) e o Parquet é escrito direto em arquivo — sem carregar o ZIP (de >1 GB), o CSV nem o Parquet inteiros na memória. Com isso entram no bronze **`tse-prestacao-contas`** e **`tse-votacao-secao`** (CSVs consolidados de vários GB), e `tse-candidaturas`/`tse-bens` usam o mesmo caminho.

## [0.18.0] - 2026-11-04

### Adicionado

- **Console SQL web** próprio e self-hosted para o bronze, no serviço `consulta` (`docker compose up -d consulta` → `http://localhost:4213`): editor SQL + lista de tabelas, rodando DuckDB nativo sobre as views do bronze, sem depender da nuvem MotherDuck (que derrubava a UI oficial — [issue #167](https://github.com/rikemorais/todeolhoaibr/issues/167)). A Local UI oficial segue disponível em `docker compose run --rm --service-ports consulta ui`, e o CLI offline em `consulta sql`.
- O painel distingue a **camada** de cada execução com um selo **Raw** (ingestão) ou **Bronze** (refino), no cartão e no detalhe. Registros antigos, sem o campo, aparecem como `raw` ([issue #168](https://github.com/rikemorais/todeolhoaibr/issues/168)).
- Bronze para os pacotes do **TSE** (ZIP com vários CSVs por UF, `;`, Latin-1): `tse-candidaturas` e `tse-bens`. Um `DecodificadorZipCsv` parametrizável passa a atender tanto a CEAP da Câmara quanto o TSE (o decodificador específico da Câmara foi removido). `tse-prestacao-contas` ficou de fora por ora (CSVs consolidados de vários GB e anos antigos em `.txt` aninhado — [issue #171](https://github.com/rikemorais/todeolhoaibr/issues/171)) ([issue #168](https://github.com/rikemorais/todeolhoaibr/issues/168)).

### Corrigido

- Refino não estoura mais a memória em ZIPs com muitas tabelas (ex.: um CSV por UF no TSE): cada Parquet decodificado vai para um **arquivo temporário em disco** assim que é gerado, e o envio ao bronze lê uma parte por vez, em vez de manter todas em memória ([issue #164](https://github.com/rikemorais/todeolhoaibr/issues/164)).

## [0.17.0] - 2026-11-04

### Adicionado

- Bronze para mais fontes (CSV solto): `senado-despesas` (CEAPS, `;`, Latin-1, com linha de preâmbulo) e `camara-votacoes` (`;`, UTF-8 com BOM). Reusa o escritor Parquet em lotes; novo `DecodificadorCsv` parametrizável (separador, linhas de preâmbulo) e remoção de BOM ([issue #168](https://github.com/rikemorais/todeolhoaibr/issues/168)).
- Disparo automático do bronze por **reconciliação agendada** ([ADR 0019](docs/adr/0019-disparo-do-bronze-por-reconciliacao.md)): o comando `refinar bronze` ganhou `--todas` (refina todas as fontes do catálogo) e `--intervalo <segundos>` (repete a reconciliação); e o serviço `agendador` no compose roda isso em laço (padrão 1h, via `TODEOLHO_REFINO_INTERVALO`), subindo junto com o stack. Como o bronze é idempotente, passadas sem mudança são quase instantâneas e não registram execução no painel ([issue #168](https://github.com/rikemorais/todeolhoaibr/issues/168)).
- Serviço `consulta` com **DuckDB** como "Athena local" ([ADR 0018](docs/adr/0018-motor-de-consulta-duckdb.md)): consulta o `bronze` em Parquet **somente-leitura**, lendo no lugar (filesystem e `s3://` via `httpfs`), sem mover os dados. Cria uma view por tabela (`bronze_<fonte>_<recurso>`) a partir da versão vigente de cada partição. A **Local UI** abre em `http://localhost:4213` (`docker compose up -d consulta`) e há um caminho **SQL offline** (`docker compose run --rm consulta sql`). Documentado no README e em `docs/refino/bronze.md` ([issue #158](https://github.com/rikemorais/todeolhoaibr/issues/158)).

## [0.16.0] - 2026-11-04

### Adicionado

- Camada **bronze** do medalhão: o novo crate `refino` decodifica a camada `raw` (descompacta ZIP, reencoda para UTF-8, parseia CSV/JSON) e grava **Parquet** no bucket `bronze`, 1:1 com a origem e sem regra de negócio. O comando `todeolho refinar bronze <fonte>` descobre os recursos ingeridos, roda no mesmo executor e painel da ingestão, e é idempotente (só reprocessa quando o `raw` muda). Primeira fonte: `camara-despesas`. As tabelas são consultadas com DuckDB ([ADR 0018](docs/adr/0018-motor-de-consulta-duckdb.md)) ([issue #151](https://github.com/rikemorais/todeolhoaibr/issues/151)).

## [0.15.0] - 2026-11-03

### Adicionado

- Fonte `ipeadata` com séries socioeconômicas regionais do Ipeadata (IPEA): renda per capita, índice de Gini, proporção de pobres e de extremamente pobres (Atlas do Desenvolvimento Humano, por município) e o IVS (Índice de Vulnerabilidade Social). Uma requisição por série traz todos os valores em JSON OData, guardados opacos em `raw/ipeadata/series/`, filtrável por `serie` ([issue #32](https://github.com/rikemorais/todeolhoaibr/issues/32)).
- Fonte sob demanda `tse-candidaturas` com o registro de candidaturas (desde 1994) e os bens declarados dos candidatos (desde 2006) do TSE, um arquivo por eleição, nacional, filtrável por `ano` e `tipo` e guardado opaco em `raw/tse/` ([issue #58](https://github.com/rikemorais/todeolhoaibr/issues/58)).
- Fonte sob demanda `ibge-pib-municipios` com o PIB dos Municípios (agregado 5938) e a população residente estimada (agregado 6579) do IBGE, por ano e por município (`N6`), desde 2001/2002. Cada ano é uma tarefa (a API dá erro ao pedir todos os municípios de vários períodos de uma vez), guardada opaca em `raw/ibge/`; o PIB per capita é derivado no refino a partir de PIB ÷ população. Filtrável por `ano` e `indicador` ([issue #30](https://github.com/rikemorais/todeolhoaibr/issues/30)).

## [0.14.0] - 2026-11-03

### Adicionado

- Fonte `bcb` com séries macroeconômicas do SGS (Sistema Gerenciador de Séries Temporais) do Banco Central: juros (SELIC, CDI), câmbio (dólar), inflação (IPCA, IPCA 12m, INPC, IGP-M), atividade (IBC-Br), crédito (inadimplência, crédito/PIB) e fiscal (dívida líquida/PIB). Uma requisição por série traz a história completa em JSON, guardada opaca em `raw/bcb/series/`, filtrável por `serie` ([issue #56](https://github.com/rikemorais/todeolhoaibr/issues/56)).

## [0.13.0] - 2026-11-03

### Adicionado

- Fonte sob demanda `tse-prestacao-contas` com a prestação de contas de campanha do TSE (receitas e despesas dos candidatos): um arquivo ZIP por eleição, nacional, desde 2002, filtrável por `ano` e guardado opaco em `raw/tse/prestacao-contas/`. A convenção de nome do arquivo varia por era eleitoral e é resolvida pela fonte ([issue #28](https://github.com/rikemorais/todeolhoaibr/issues/28)).

## [0.12.0] - 2026-11-03

### Adicionado

- Fonte sob demanda `senado-despesas` com as despesas da CEAPS (Cota para o Exercício da Atividade Parlamentar dos Senadores): um arquivo CSV por ano, nacional, desde 2008, filtrável por `ano` e guardado opaco em `raw/senado/despesas/` ([issue #26](https://github.com/rikemorais/todeolhoaibr/issues/26)).

## [0.11.0] - 2026-11-03

### Mudado

- O menu carrega muito mais rápido no passo "Consultando o tamanho dos dados...": a estimativa ganha dois modos (`PrecisaoDaEstimativa`). O menu usa o modo rápido, em que uma cópia completa e íntegra na Raw vale zero sem consultar a origem (o mesmo atalho que as fontes de API já usavam), evitando um `HEAD` por arquivo num host limitado (a Câmara serve 5 req/s e `camara-votacoes` tem centenas de arquivos); a checagem de espaço em disco antes de executar mantém o modo exato (confere a origem), só nas tarefas escolhidas. As fontes passam a ser estimadas em paralelo. A idempotência não muda: a rodada real continua fazendo a pré-checagem condicional por arquivo ([issue #137](https://github.com/rikemorais/todeolhoaibr/issues/137)).

## [0.10.1] - 2026-11-03

### Corrigido

- Menu de fontes alinha a coluna `Falta baixar` uma coluna depois do nome de fonte mais longo: a largura da coluna de nomes passa a ser calculada em tempo de execução, então nomes longos (`senado-processos-detalhes`, `camara-tramitacoes`) não empurram mais os valores para a direita ([issue #134](https://github.com/rikemorais/todeolhoaibr/issues/134)).

## [0.10.0] - 2026-11-02

### Adicionado

- Indicadores macro do IBGE na fonte `ibge`: IPCA e INPC (inflação mensal), PIB trimestral e PNAD Contínua (desocupação por UF), baixando a série completa do SIDRA (agregados v3) e gravando a resposta opaca na Raw para o refino ([issue #57](https://github.com/rikemorais/todeolhoaibr/issues/57)).
- Percentual de progresso por tarefa em tempo real no painel: tarefas que baixam arquivo reportam bytes baixados e total durante a execução, e o painel mostra a porcentagem, uma barra e `baixado de total` enquanto a tarefa roda ([issue #128](https://github.com/rikemorais/todeolhoaibr/issues/128)).

### Mudado

- `camara-tramitacoes` passa a baixar os arquivos anuais em lote dos Dados Abertos (um por ano) em vez de percorrer a API proposição a proposição, trocando centenas de milhares de requisições por ~26 downloads e tornando a rodada incremental: o ano sem mudança responde `304` (`If-None-Match`) e é pulado ([issue #127](https://github.com/rikemorais/todeolhoaibr/issues/127)).

### Corrigido

- Painel volta a mostrar `-` (em vez de `0 B`) no volume das tarefas de API em execução, que não baixam arquivo e não têm tamanho total conhecido ([issue #131](https://github.com/rikemorais/todeolhoaibr/issues/131)).

## [0.9.0] - 2026-11-02

### Adicionado

- Fonte sob demanda `camara-tramitacoes` preserva os históricos oficiais das proposições por ano, incluindo órgão, data, situação e descrição dos passos ([issue #113](https://github.com/rikemorais/todeolhoaibr/issues/113)).

## [0.8.0] - 2026-11-02

### Adicionado

- Fonte sob demanda `senado-processos-detalhes` preserva detalhes, informes e movimentações por processo para registrar o histórico de tramitação e os eventos de arquivamento publicados pela API ([issue #112](https://github.com/rikemorais/todeolhoaibr/issues/112)).

## [0.7.0] - 2026-11-02

### Adicionado

- Fontes sob demanda `senado-processos` (matérias desde 1826) e `senado-parlamentares` (composição por legislatura desde 1991), com respostas preservadas na Raw para o refino ([issue #94](https://github.com/rikemorais/todeolhoaibr/issues/94)).

## [0.6.0] - 2026-11-02

### Adicionado

- Gravações S3 de objetos, início e conclusão multipart repetem falhas de rede e respostas transitórias com tentativas limitadas, backoff e jitter; erros permanentes falham imediatamente (issue #90).

### Corrigido

- Remove da Raw o objeto da parte quando a transferência falha na validação final de tamanho (issue #97).

## [0.5.1] - 2026-11-02

### Corrigido

- As fontes anuais da Câmara passam a incluir o ano corrente automaticamente; um arquivo do ano corrente ainda não publicado é ignorado sem ocultar a ausência de arquivos históricos (issue #95).

## [0.5.0] - 2026-11-02

### Adicionado

- O menu mostra quanto falta baixar por fonte e um total conhecido, sinalizando valores parciais; após a escolha, exibe a estimativa das tarefas selecionadas (issue #96).
- Enter no primeiro prompt do menu seleciona todas as fontes; `q` sai sem executar (issue #96).
- Perfil HTTP opcional por fonte, com credencial por variável de ambiente e limite de requisições compartilhado por host (issue #70).

### Corrigido

- O menu deixa de consultar metadados de todas as fontes `sob_demanda` ao abrir e mostra um erro específico para combinações de filtros sem tarefas (issue #89).
- Respostas HTTP 401/403 agora falham claramente também nas consultas de metadados de arquivos, sem serem tratadas como tamanho desconhecido.

## [0.4.0] - 2026-11-02

### Adicionado

- Site de documentação no GitHub Pages com Material for MkDocs, versionado com `mike`, incluindo a referência de API (`rustdoc`) sob `/api` ([issue #83](https://github.com/rikemorais/todeolhoaibr/issues/83)).
- Fonte `camara-despesas` (sob demanda) com as despesas da CEAP (Cota para o Exercício da Atividade Parlamentar): um arquivo anual, desde 2008, filtrável por `ano` e guardado em `raw/camara/despesas/` ([issue #25](https://github.com/rikemorais/todeolhoaibr/issues/25)).
- Fonte `camara-votacoes` (sob demanda) com proposições e votações nominais da Câmara ("quem votou em quê"): arquivos anuais em lote de 7 conjuntos (incluindo `votacoesVotos`, o voto de cada deputado), desde 2001, filtráveis por `ano` e `conjunto`, guardados em `raw/camara/votacoes/` ([issue #86](https://github.com/rikemorais/todeolhoaibr/issues/86)).
- Fonte `senado-votacoes` (sob demanda) com matérias e votações nominais do Senado ("quem votou em quê"): uma consulta por ano ao serviço `votacao` (com o voto de cada senador embutido), desde 1991, filtrável por `ano` e guardada em `raw/senado/votacoes/` ([issue #87](https://github.com/rikemorais/todeolhoaibr/issues/87)).

### Corrigido

- Download de arquivos grandes resiliente a quedas de conexão: ao cair no meio, a `TarefaDeArquivo` **retoma** do byte exato já recebido via HTTP `Range` (com `If-Range` para garantir o mesmo arquivo), em vez de recomeçar do zero. Resolve as falhas dos CSVs grandes da Câmara (30–56 MB), que a origem derruba no meio, sem perder nem duplicar byte ([issue #92](https://github.com/rikemorais/todeolhoaibr/issues/92)).

## [0.3.0] - 2026-11-01

### Adicionado

- Publicação do ponteiro `_ultima_versao.json` resistente a sobrescrita por uma rodada mais antiga, comparando o início da execução ([issue #16](https://github.com/rikemorais/todeolhoaibr/issues/16)).
- Gate de cobertura de documentação no CI (`cargo doc` com `-D warnings`) e `#![warn(missing_docs)]` nas bibliotecas, para a documentação não regredir ([issue #20](https://github.com/rikemorais/todeolhoaibr/issues/20)).
- Gate de formatação e lint no CI (`cargo fmt --check` e `cargo clippy -D warnings`), barrando PRs fora do padrão ([issue #4](https://github.com/rikemorais/todeolhoaibr/issues/4)).
- Diagramas de arquitetura e de fluxo da ingestão (Archify), em `docs/diagramas`, com prévia no README ([issue #7](https://github.com/rikemorais/todeolhoaibr/issues/7)).

## [0.2.1] - 2026-10-31

### Adicionado

- Documentação completa (100%) da API pública das bibliotecas (`nucleo`, `infraestrutura`, `fontes`, `painel`) e os primeiros doctests verificados pelo `cargo test` ([issue #6](https://github.com/rikemorais/todeolhoaibr/issues/6)).

## [0.2.0] - 2026-10-31

Pré-checagem idempotente: rodar o processo de novo não rebaixa nem duplica o que já está na camada Raw.

### Adicionado

- Pré-checagem idempotente na ingestão: antes de buscar, confere a Raw (manifesto de procedência com SHA-256 e o tamanho de cada parte) e só (re)ingere o que está **ausente, menor, desatualizado ou incompleto** ([issue #13](https://github.com/rikemorais/todeolhoaibr/issues/13)). A integridade das partes é por tamanho (detecta ausência e truncamento), barata em memória; ver [ADR 0012](docs/adr/0012-pre-checagem-idempotente.md).
- Requisição condicional (`If-None-Match`) por página nas APIs paginadas: 304 reaproveita a página guardada, 200 reingere; páginas inalteradas não são reescritas.
- O painel mostra, por tarefa, se foi reaproveitada ou reingerida e o motivo.

### Alterado

- O porto `ClienteHttp` ganha requisição condicional e `ArmazenamentoRaw` ganha a consulta de `tamanho` (HEAD), usada na verificação de integridade sem baixar o arquivo. Ver [ADR 0012](docs/adr/0012-pre-checagem-idempotente.md).

## [0.1.0] - 2026-10-31

Primeira versão, com a ingestão de dados públicos para a camada Raw.

### Adicionado

- Fontes de dados `ibge` (regiões, estados e municípios), `camara` (legislaturas, partidos e deputados em exercício) e `tse` (votos por seção eleitoral, de 1998 a 2026).
- Menu de terminal para escolher o que ingerir, com o tamanho de cada fonte, ano e UF.
- Filtros por etiqueta na linha de comando, como `--filtro ano=2024 --filtro uf=AC`.
- Verificação de espaço em disco antes de cada execução, recusando o que não cabe.
- Camada Raw imutável, compatível com S3, com manifesto de procedência e SHA-256 de cada arquivo.
- Transferência de arquivos grandes sem carregá-los na memória, com novas tentativas e limpeza de transferências interrompidas.
- Retomada de cargas interrompidas, sem baixar de novo o que não mudou na origem.
- Painel web de execuções em `http://localhost:8080`, com histórico, DAG e erros de cada tarefa.
- Navegador de arquivos em `http://localhost:7071`.
- Ambiente completo em Docker, sem necessidade de conta ou token.
- Registros de decisão de arquitetura em `docs/adr`.
- Integração contínua que roda os testes do workspace (`cargo test`) em cada pull request.
- Dependabot para `cargo`, `github-actions` e `docker`, com atualizações semanais.
- Licença dual MIT OR Apache-2.0.
