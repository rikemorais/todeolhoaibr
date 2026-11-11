# Histórico de mudanças

Todas as mudanças relevantes do projeto ficam registradas aqui.

O formato segue o [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/), e o projeto segue o
[Versionamento Semântico](https://semver.org/lang/pt-BR/). Veja o [ADR 0011](docs/adr/0011-versionamento-e-commits.md).

## [Não lançado]

## [0.69.0] - 2026-11-11

### Adicionado

- Contrato Silver para receitas de candidatos do TSE em 2020, preservando ocorrências e linhagem Bronze (issue #189); smoke integral publicou 2.339.073 linhas.

## [0.68.0] - 2026-11-11

### Adicionado

- Contrato Silver para despesas pagas do TSE em 2020, preservando ocorrências com linhagem Bronze (issue #189); smoke integral publicou 3.086.924 linhas.

## [0.67.0] - 2026-11-11

### Adicionado

- Contrato Silver para despesas contratadas do TSE em 2020, preservando ocorrências repetidas com linhagem Bronze (issue #189); smoke integral publicou 4.107.531 linhas.

## [0.66.0] - 2026-11-11

### Adicionado

- Contrato Silver para despesas pagas do TSE em 2022, preservando ocorrências com linhagem Bronze (issue #189); smoke integral publicou 2.411.963 linhas.

## [0.65.0] - 2026-11-10

### Adicionado

- Contrato Silver para despesas contratadas do TSE em 2022, preservando variantes e ocorrências repetidas com linhagem Bronze (issue #189); smoke integral publicou 2.209.835 linhas.

## [0.64.0] - 2026-11-10

### Adicionado

- Contrato Silver para receitas de candidatos do TSE em 2022, preservando ocorrências repetidas, tipando valores e datas e registrando linhagem Bronze (issue #189); smoke integral publicou 674.944 linhas.

## [0.63.0] - 2026-11-10

### Adicionado

- Contrato Silver para receitas de doador originário do TSE em 2022, preservando variantes, documento como texto e linhagem Bronze (issue #189); smoke integral publicou 199.241 linhas.

## [0.62.0] - 2026-11-10

### Adicionado

- Contrato Silver para receitas de doador originário do TSE em 2024, preservando variantes, documento como texto e linhagem Bronze (issue #189); smoke integral publicou 534.344 linhas.

## [0.61.0] - 2026-11-10

### Adicionado

- Contrato Silver para despesas contratadas do TSE em 2024, preservando variantes com linhagem Bronze (issue #189); smoke integral publicou 4.452.310 linhas.

## [0.60.0] - 2026-11-10

### Adicionado

- Contrato Silver de receitas de candidatos do TSE em 2024, preservando variantes e tipando sequência, data e valor com linhagem da origem (issue #189); smoke integral publicou 2.041.059 linhas.
- Corrige a projeção de linhagem na consulta de despesas pagas do TSE e preserva datas ausentes de receitas como nulas; smoke integral publicado junto com despesas pagas (3.719.650 linhas).

## [0.59.0] - 2026-11-10

### Adicionado

- Fonte `remuneracao-estadual-go` para ingestão sob demanda do CSV mensal mais recente listado no catálogo de Goiás, preservado em fluxo na Raw (issue #75, PR #326). A cobertura de outras UFs e do subsídio do governador permanece pendente.

## [0.58.0] - 2026-11-10

### Adicionado

- Fonte `remuneracao-estadual-mg` para ingestão sob demanda do CSV mensal mais recente de remuneração do Executivo de Minas Gerais, preservado em fluxo na Raw (issue #75, PR #324). A cobertura do subsídio do governador e das demais UFs continua pendente.

## [0.57.0] - 2026-11-10

### Adicionado

- Fonte `transparencia-servidores-ativos` para ingestão mensal sob demanda de arquivos SIAPE e BACEN de remuneração de servidores civis ativos do Executivo Federal, preservados na Raw (issue #74, PR #320).

## [0.56.0] - 2026-11-10

### Adicionado

- Fonte `transparencia-beneficios` para ingestão sob demanda de Bolsa Família, Auxílio Brasil, Novo Bolsa Família e BPC/LOAS por município e competência, preservada na Raw (issue #52, PR #316).

## [0.55.0] - 2026-11-10

### Adicionado

- Fonte `remuneracao-municipal-tce-ce` para ingestão sob demanda dos totais agregados das folhas do TCE-CE, particionados por município na competência 2025-12 (issue #76, PR #314).

## [0.54.0] - 2026-11-10

### Adicionado

- Contratos SQL Silver para IBGE localidades: `ibge_regioes` (5 regiões), `ibge_estados` (27 UFs com região aninhada), `ibge_municipios` (5.571 municípios com hierarquia completa) (issue #187, PR #311).
- Contrato SQL Silver para Ipeadata: parsing de data ISO/BR, valor decimal(38,9) com original preservado, chave (serie, territorio, data_texto) (issue #187, PR #311).
- Esquemas Arrow tipados e adaptadores `TarefaDeSilverIbgeLocalidades` e `TarefaDeSilverIpeadata` com testes de publicação e idempotência (issue #187, PR #311).
- Subcomandos CLI `ibge-regioes`, `ibge-estados`, `ibge-municipios` e `ipeadata` integrados ao `--todas` (issue #187, PR #311).

## [0.53.2] - 2026-11-10

### Refatoração

- Move as 8 transformações Arrow manuais restantes para `#[cfg(test)]`; o binário de produção executa somente SQL/DataFusion (issue #244, PR #306).

### Testes

- Teste de inventário SQL bidirecional: todo `.sql` em `sql/silver/` tem `include_str!` correspondente e vice-versa (issue #244, PR #307).

### Documentação

- Documenta convenções SQL, tipos/sentinelas, placeholders, UDFs, versionamento da regra, fixtures, execução local, processo de revisão e inventário dos 24 contratos (issue #244, PR #308).

## [0.53.1] - 2026-11-10

### Corrigido

- Serializa por recurso a conferência final do Bronze e a publicação Silver no processo; preserva o primeiro ponteiro datado quando os instantes empatam (issue #185, PR #298).
- Reporta recusa de ponteiro como falha em Raw, Bronze e Silver; distingue tarefas Silver irmãs pelo destino e registra sessões e votos do Senado separadamente na CLI (issue #185, PR #298).
- Alinha o `Cargo.lock` à versão 0.53.0 dos crates locais, restaurando os checks com `--locked` após a release anterior (PR #298).

### Documentação

- Especifica tipos, nulos, normalização, duplicatas e tabelas internas das 26 entradas Bronze; define limiares de qualidade e falha parcial no ADR Silver (issue #185, PRs #297 e #299).
- Registra o limite atual de concorrência entre processos e da consulta Silver diante de uma nova versão Bronze na issue #300 (PR #297).

## [0.53.0] - 2026-11-10

### Adicionado

- Contrato de escrita condicional no trait `ArmazenamentoRaw` via `gravar_condicional`; o adaptador S3 ainda usa a implementação padrão sem atomicidade entre processos (issue #300, PR #304).
- Validação de vigência do Bronze antes de publicar Silver via `SnapshotBronze::ainda_vigente()` (issue #300, PR #304).
- Testes de duplicatas Bronze, mudança do ponteiro durante a transformação e linhagem na tarefa SQL (issue #300, PR #304).

## [0.52.0] - 2026-11-09

### Adicionado

- Projeta colunas de linhagem Bronze (`_tdoa_origem_manifesto`, `_tdoa_origem_parte`, `_tdoa_origem_ordinal`) em toda saída Silver — 24 contratos SQL, 17 schemas e validação no publisher (issue #300, PR #303).

## [0.51.4] - 2026-11-09

### Documentação

- Mede completude e colisões das chaves candidatas de receitas e despesas partidárias do TSE 2016 no snapshot Bronze (issue #185, PR #295).

## [0.51.3] - 2026-11-09

### Documentação

- Identifica a cobertura adicional `BR` e `DF` dos lançamentos partidários TSE 2016 e alinha a orientação de fonte Silver para o snapshot validado (issue #185, PR #293).

## [0.51.2] - 2026-11-09

### Documentação

- Valida schemas, contagens e conteúdo dos recortes TSE de 2016 a 2022 e registra as divergências de partidos em 2016 (issue #185, PR #291).

## [0.51.1] - 2026-11-09

### Documentação

- Mede chaves candidatas para as seis famílias regulares do TSE 2014 e documenta a semântica não definida dos arquivos `_sup` (issue #185, PR #289).

## [0.51.0] - 2026-11-09

### Adicionado

- Organiza o catálogo do console SQL em grupos recolhíveis por camada Bronze/Silver e por fonte, preservando os nomes SQL das views (issue #286, PR #287).

### Documentação

- Registra a validação das prestações de contas TSE de 2014 e as pendências de conteúdo, chaves e tabelas suplementares (issue #185, PR #285).

## [0.50.2] - 2026-11-09

### Documentação

- Validados schemas e contagens de linhas das prestações de contas TSE de 2014; separadas as pendências de conteúdo, chaves e tabelas suplementares (#185, PR #283).

## [0.50.1] - 2026-11-09

### Corrigido

- Sincronizadas as versões dos crates locais no `Cargo.lock` com a release v0.50.0, restaurando os checks com `--locked` (PR #281).

### Documentação

- Inventário rastreável dos manifestos Bronze de prestação de contas do TSE de 2014 a 2022, como avanço dos contratos Silver da issue #185 (PR #280).

## [0.50.0] - 2026-11-09

### Adicionado

- Decodificação bronze 100% em fluxo: CSV e ZIP-CSV são lidos em streaming sem carregar o arquivo inteiro em memória (issue #164).
- `LeitorAutoEncoding`: auto-detecção de encoding por amostragem dos primeiros 16 KB — UTF-8 passa direto, Latin-1 é transcodificado via `encoding_rs_io`.
- `partes_de_csv_em_fluxo`: caminho streaming para CSV "solto" (ex.: CEAPS do Senado).
- `DecodificadorCsv::decodificar_arquivo`: lê do disco em fluxo em vez de carregar em memória.
- `descartar_uma_linha`: descarte de preâmbulo com buffer fixo (sem alocar a linha descartada).

### Alterado

- `DecodificadorZipCsv` unificado: `novo()` e `em_fluxo()` produzem o mesmo decodificador (sempre streaming).
- `partes_de_zip_csv` agora delega para `partes_de_zip_csv_em_fluxo`.

## [0.49.0] - 2026-11-09

### Adicionado

- Módulo `nucleo::orfas` para recuperação de partes Raw órfãs após cancelamento ou erro ambíguo (issue #269).
- `verificar_manifesto_ou_limpar` verifica se o manifesto realmente foi persistido antes de decidir o rollback, devolvendo `ResultadoVerificacao` com três estados (confirmado, ausente, ilegível).
- `limpar_orfas_do_recurso` varre prefixo e remove partes desprotegidas, excluindo a execução corrente e protegendo execuções com manifesto ilegível.
- `listar_prefixo` na porta `ArmazenamentoRaw` (default vazio) e implementado no adapter S3.
- Varredura pré-execução de órfãs em `ingestao_http`.

### Corrigido

- Erro na verificação de manifesto após escrita ambígua agora é logado (antes era silenciado com `.ok()`).

## [0.48.0] - 2026-11-09

### Adicionado

- Método `remover_lote` na porta `ArmazenamentoRaw` com concorrência limitada, deadline global e coleta de falhas individuais via `FalhaDeRemocao` (issue #270).

### Corrigido

- `remover()` no adapter S3 agora retenta falhas transitórias com backoff exponencial (4 tentativas).
- Rollback de partes HTTP usa `remover_lote` (4 paralelos, 30s deadline) em vez de loop sequencial.
- Chaves removidas com sucesso não são mais reportadas como falha quando o deadline expira.
- Logs de retry identificam a operação (PUT, DELETE, POST) em vez de "gravação" genérica.

## [0.47.4] - 2026-11-09

### Corrigido

- Valida respostas JSON gzip após decodificação limitada, preserva os bytes HTTP originais na Raw e decodifica gzip em fluxo antes do Bronze; rejeita codificações não suportadas ou truncadas (issue #264).

## [0.47.3] - 2026-11-09

### Corrigido

- Faz até três tentativas totais para baixar uma resposta JSON sintaticamente válida, sem publicar tentativas incompletas (issue #263).

## [0.47.2] - 2026-11-09

### Corrigido

- Remove partes novas sem manifesto quando uma execução paginada falha, preservando partes reutilizadas (issue #262).

## [0.47.1] - 2026-11-09

### Corrigido

- Rejeita arquivos HTTP sem tamanho declarado quando o EOF não tem framing verificável; valida JSON grande em fluxo antes de publicar manifesto (issue #261).

## [0.47.0] - 2026-11-08

### Alterado

- Migra candidaturas, bens, votação por seção e despesas pagas do TSE para contratos SQL/DataFusion, preservando schemas, validação de chaves, aliases históricos, filtros e a deduplicação exata das despesas pagas (issue #243).
- Valida unicidade de candidaturas e bens por agregação escalar das chaves, evitando a janela global sobre linhas completas e reduzindo o pico de RSS nos smokes TSE.
- Compacta a validação da chave da votação por seção em uma chave escalar com prefixos de tamanho e limita o executor por recurso a 256 MiB de memória e 8 GiB de spill.
- Ajusta despesas pagas TSE para pool DataFusion de 256 MiB e spill de 8 GiB, com medição em cgroup Docker de 2 GiB.

### Corrigido

- Rejeita respostas JSON HTTP truncadas antes de publicar manifestos Raw e recupera partes inválidas referenciadas por 304 (issue #146).

## [0.46.0] - 2026-11-07

### Adicionado

- Adiciona executor SQL/DataFusion em fluxo e tarefa reutilizável para contratos Silver, com limites de recursos e linhagem explícita (issue #238).

### Alterado

- Registra a arquitetura para SQL versionado executado por Apache DataFusion nas transformações Silver; executor e migração dos contratos serão implementados gradualmente (ADR 0020, épico #236).
- Migra os contratos BCB/SGS e agregados IBGE para SQL/DataFusion, mantém a paridade e o schema publicados, e reprocessa uma vez os dados existentes com regras v2 (issue #239).
- Migra deputados, partidos e legislaturas da Câmara e os processos e detalhes do Senado para SQL/DataFusion, preservando schemas, campos JSON dinâmicos e validação de chaves com regras v2 (issue #240).
- Migra despesas CEAP, tramitações anuais e os sete conjuntos de votações da Câmara para SQL/DataFusion, preservando registros, schema de origem e validação de chaves (issue #241).
- Migra despesas CEAPS, parlamentares, sessões e votos nominais do Senado para SQL/DataFusion; adiciona UDFs genéricas de leitura JSON para o DataFusion 55.2, expande envelopes atuais e legados com `UNNEST` e mantém paridade com as transformações anteriores (issue #242).

## [0.45.0] - 2026-11-06

### Adicionado

- Publica despesas pagas do consolidado BRASIL do TSE em 2024 na Silver, removendo apenas cópias integrais e rejeitando conflitos de chave (issue #189).

## [0.44.0] - 2026-11-06

### Adicionado

- Publica votação por seção do TSE na Silver com chave composta validada, filtros por ano/UF e registro original preservado. Recursos com cabeçalho Bronze incompatível ficam documentados e excluídos (issue #189).

## [0.43.0] - 2026-11-06

### Adicionado

- Publica os bens declarados nacionais do TSE na Silver com chave do candidato e ordem do bem validadas, datas e valores tipados e campos originais preservados (issue #189).

## [0.42.0] - 2026-11-06

### Adicionado

- Publica os registros nacionais de candidaturas do TSE na Silver, com chave composta validada, datas e idade tipadas e preservação dos campos originais (issue #189).

## [0.41.0] - 2026-11-06

### Adicionado

- Publica despesas anuais CEAPS do Senado na Silver com mês, data e valor tipados, validação do código de documento e preservação de registros incompletos ou afetados por CSV malformado (issue #188).

## [0.40.0] - 2026-11-06

### Adicionado

- Publica o histórico anual de tramitações da Câmara na Silver, preservando linhas repetidas quando não há chave de negócio única comprovada (issue #188).

## [0.39.0] - 2026-11-06

### Adicionado

- Publica despesas anuais da CEAP da Câmara na Silver com valores e datas tipados, preservação dos campos originais e sem deduplicação por ID não único (issue #188).

## [0.38.1] - 2026-11-06

### Corrigido

- Reprocessa uma vez as listagens anuais de processos do Senado para substituir Parquets vazios sem colunas por manifestos sem partes, que o DuckDB consegue ignorar.

## [0.38.0] - 2026-11-06

### Adicionado

- Publica o catálogo anual de processos do Senado na Silver, tipando ID e ano e preservando o registro de origem em JSON (parte da issue #188).
- Lê Parquets Bronze legados sem colunas e sem linhas como recursos vazios, usando o schema declarado pelo contrato Silver.

## [0.37.0] - 2026-11-06

### Adicionado

- Publica detalhes de processos do Senado na Silver com ID textual, ano tipado, payload integral preservado e validação de unicidade por recurso anual (parte da issue #188).
- Trata manifestos Bronze válidos sem partes como recursos sem dados, permitindo que reconciliações Silver pulem anos vazios sem confundi-los com corrupção.

## [0.36.1] - 2026-11-06

### Corrigido

- Limita a reconciliação IBGE Silver às fontes agregadas que possuem contrato e preserva votos da Câmara com chave candidata incompleta, reprocessando-os pela regra v2 (issue #188).

## [0.36.0] - 2026-11-06

### Adicionado

- Publica os votos nominais individuais do Senado como relação Silver `votos/ano=AAAA`, preservando a relação de sessões e cada registro de origem (parte da issue #188).

## [0.35.0] - 2026-11-06

### Adicionado

- Adiciona o contrato Silver das votações anuais do Senado, com validação de `codigoSessaoVotacao` e preservação integral da lista de votos aninhada; integra a CLI e a reconciliação periódica (parte da issue #188).

## [0.34.0] - 2026-11-06

### Adicionado

- Adiciona validação de chaves e publicação Silver preservando os registros dos sete conjuntos anuais de votações da Câmara; integra a fonte à CLI e à reconciliação periódica (parte da issue #188).

## [0.33.0] - 2026-11-06

### Adicionado

- Adiciona o contrato Silver da fotografia vigente de deputados em exercício da Câmara, com validação de IDs e integração à CLI/reconciliação ([issue #186](https://github.com/rikemorais/todeolhoaibr/issues/186)).

## [0.32.0] - 2026-11-06

### Adicionado

- Adiciona o contrato Silver de legislaturas da Câmara, com IDs sequenciais, períodos tipados e ano eleitoral; integra `camara-legislaturas` à CLI e à reconciliação Silver ([issue #186](https://github.com/rikemorais/todeolhoaibr/issues/186)).

## [0.31.0] - 2026-11-06

### Adicionado

- Adiciona o contrato Silver do cadastro de partidos da Câmara, valida IDs únicos por recurso e integra a fonte à execução individual, `--todas` e reconciliação periódica ([issue #186](https://github.com/rikemorais/todeolhoaibr/issues/186)).

## [0.30.0] - 2026-11-06

### Adicionado

- Adiciona o contrato Silver de parlamentares por legislatura e integra a publicação ao comando `refinar silver` ([issue #186](https://github.com/rikemorais/todeolhoaibr/issues/186)).

### Alterado

- Compartilha o fluxo de publicação Silver tipada entre BCB, IBGE e Senado.

### Alterado

- Compartilha o fluxo de publicação Silver tipada entre BCB, IBGE e Senado.

## [0.29.0] - 2026-11-06

### Adicionado

- Publica agregados IBGE na Silver e permite reconciliá-los pela CLI, individualmente ou junto das demais fontes Silver suportadas ([issue #186](https://github.com/rikemorais/todeolhoaibr/issues/186)).

## [0.28.0] - 2026-11-06

### Adicionado

- Adiciona o contrato de transformação Silver para agregados IBGE, preservando o valor original e expondo conversão decimal opcional ([issue #186](https://github.com/rikemorais/todeolhoaibr/issues/186)).

## [0.27.0] - 2026-11-06

### Adicionado

- Gera views `silver_*` no console DuckDB usando somente os manifestos Silver apontados como vigentes, ao lado das views Bronze ([issue #186](https://github.com/rikemorais/todeolhoaibr/issues/186)).

## [0.26.0] - 2026-11-06

### Adicionado

- Registra execuções Silver BCB no painel e adiciona `--todas` e `--intervalo` com pré-checagem para não registrar passadas vazias ([issue #186](https://github.com/rikemorais/todeolhoaibr/issues/186)).

## [0.25.0] - 2026-11-06

### Adicionado

- Publica a primeira tabela Silver BCB/SGS via `refinar silver bcb`, com idempotência por versão/regra, Parquet tipado, manifesto e ponteiro; aborta se o snapshot Bronze mudar durante a transformação ([issue #186](https://github.com/rikemorais/todeolhoaibr/issues/186)).

## [0.24.0] - 2026-11-06

### Adicionado

- Define o primeiro transformador Silver BCB/SGS, com data `Date32` e valor `Decimal128(18,6)`, conversão decimal exata e erro explícito para registros inválidos ([issue #186](https://github.com/rikemorais/todeolhoaibr/issues/186)).

## [0.23.0] - 2026-11-06

### Adicionado

- Escritor Parquet em lotes com schema Arrow tipado para os futuros contratos Silver; valida schema exato por lote e preserva datas, decimais e nulos em arquivo, sem converter para texto ([issue #186](https://github.com/rikemorais/todeolhoaibr/issues/186)).

## [0.22.5] - 2026-11-05

### Documentação

- Confere as 168 partes de prestação de contas TSE 2010 contra os membros do ZIP e reconcilia receitas e despesas de comitês de 2012 entre UFs e `_brasil`. Registra linhas exclusivas, formatos de data e colisões da chave candidata para orientar os contratos Silver ([issue #208](https://github.com/rikemorais/todeolhoaibr/issues/208)).

## [0.22.4] - 2026-11-05

### Documentação

- Inventaria as seis famílias de prestação de contas TSE em 2010 e 2012 nas versões Bronze vigentes, com volumes, diferenças entre partes estaduais e consolidadas e formatos de data por conjunto. Mantém pendentes a chave de lançamento e o leiaute posicional de receitas partidárias de 2012 ([issue #185](https://github.com/rikemorais/todeolhoaibr/issues/185)).

## [0.22.3] - 2026-11-05

### Documentação

- Define o contrato candidato dos quatro conjuntos de contas TSE 2008 e exclui o TXT `leiaute-2008` do catálogo tabular Silver; mede linhas, campos vazios e conversões e registra divergências entre o leiaute do ZIP e os cabeçalhos físicos ([issue #185](https://github.com/rikemorais/todeolhoaibr/issues/185)).

## [0.22.2] - 2026-11-05

### Documentação

- Valida os oito conjuntos de prestação de contas TSE 2004/2006 do Bronze vigente, medindo schemas, duplicatas exatas, datas e valores vazios e conversibilidade. Registra contratos candidatos por ano e tabela sem presumir equivalência entre cabeçalhos distintos ([issue #185](https://github.com/rikemorais/todeolhoaibr/issues/185)).

## [0.22.1] - 2026-11-05

### Documentação

- Valida separadamente os quatro conjuntos de prestação de contas TSE 2002 no Bronze vigente: schemas, grãos candidatos, tipos, nulos e duplicatas. Registra a política candidata de preservar cada linha com linhagem técnica até que a chave de negócio seja comprovada ([issue #185](https://github.com/rikemorais/todeolhoaibr/issues/185)).

## [0.22.0] - 2026-11-05

### Adicionado

- Snapshot de leitura da versão Bronze vigente: valida ponteiro, manifesto, fonte e recurso; baixa apenas partes listadas em fluxo e confere tamanho e SHA-256 antes de entregar lotes à Silver ([issue #186](https://github.com/rikemorais/todeolhoaibr/issues/186)).

## [0.21.0] - 2026-11-05

### Adicionado

- Leitor em lotes das partes Bronze com referência técnica por manifesto, parte, tabela e ordinal da linha. Preserva variantes e duplicatas do TSE para os futuros contratos Silver, sem reprocessar o Bronze ([issue #186](https://github.com/rikemorais/todeolhoaibr/issues/186)).

## [0.20.6] - 2026-11-05

### Documentação

- Define a política candidata das contas TSE 2024: remover apenas 79 cópias exatas em despesas pagas, preservar variantes nos demais conjuntos e rastrear a linha de origem; corrige que o documento, e não a data, eleva a cardinalidade de doador originário ([issue #185](https://github.com/rikemorais/todeolhoaibr/issues/185)).

## [0.20.5] - 2026-11-05

### Documentação

- Rejeita as chaves sequenciais simples dos quatro conjuntos de prestação de contas do TSE em 2024 após validar colisões nos consolidados locais; documenta variantes de atributos e a necessidade de contratos por conjunto e ano ([issue #185](https://github.com/rikemorais/todeolhoaibr/issues/185)).

## [0.20.4] - 2026-11-05

### Documentação

- Valida a chave candidata da votação por seção do TSE em AC e RO nos 15 anos locais e em SP/2024, mantendo explícita a necessidade de conferir as demais UFs e versões ([issue #185](https://github.com/rikemorais/todeolhoaibr/issues/185)).

## [0.20.3] - 2026-11-05

### Documentação

- Valida chaves candidatas das candidaturas e dos bens do TSE em consolidados históricos, identifica colisões das chaves curtas e linhas divergentes nas partes por UF de 2000; registra a pesquisa oficial que mantém pendente o leiaute das receitas partidárias de 2012 ([issue #185](https://github.com/rikemorais/todeolhoaibr/issues/185)).

## [0.20.2] - 2026-11-05

### Documentação

- Valida a chave candidata (`recurso`, `data`) nas 11 séries BCB disponíveis no bronze local e registra o bloqueio de leiaute oficial para as colunas posicionais de receitas partidárias de 2012 do TSE ([issue #185](https://github.com/rikemorais/todeolhoaibr/issues/185)).

## [0.20.1] - 2026-11-05

### Corrigido

- Os 27 TXT de `receitas_partidos_2012_<sigla>` sem cabeçalho preservam a primeira linha como dado no bronze e recebem colunas posicionais. O recurso de prestação de contas de 2012 ganha um marcador de regra para ser reprocessado uma vez pelo agendador, mesmo sem mudança na Raw ([issue #194](https://github.com/rikemorais/todeolhoaibr/issues/194)).

## [0.20.0] - 2026-11-05

### Adicionado

- Selos de camada no painel com cores próprias: Bronze marrom médio, Silver azul, Gold amarelo sol e Diamond roxo; Raw permanece cinza. Os nomes e estilos das camadas futuras ficam prontos para aparecer quando as respectivas execuções forem implementadas.

## [0.19.3] - 2026-11-05

### Documentação

- Registra a validação local de chaves candidatas da silver para séries, território, fontes legislativas e recortes do TSE; identifica chaves não únicas e um cabeçalho antigo do TSE que exige investigação antes da materialização ([issue #185](https://github.com/rikemorais/todeolhoaibr/issues/185)).

## [0.19.2] - 2026-11-05

### Documentação

- Esclarece como processar o bronze em uma instalação nova e após atualizar uma instalação que já tem Raw, incluindo o agendador automático, a passada manual e a conferência dos Parquets.

## [0.19.1] - 2026-11-05

### Documentação

- Início do contrato da camada silver: ADR 0020 proposto, glossário do refino e matriz das 26 entradas bronze com grão e chaves candidatas, evidências das amostras locais e validações pendentes ([issue #185](https://github.com/rikemorais/todeolhoaibr/issues/185)).

## [0.19.0] - 2026-11-05

### Adicionado

- Bronze para fontes em **JSON de API**, via um `DecodificadorJson` genérico (array de registros no topo ou sob uma chave; campos viram colunas de texto, aninhados viram JSON compacto): `bcb`, `ipeadata`, `camara-tramitacoes`, `camara-deputados`, `camara-partidos`, `camara-legislaturas`, `ibge-estados`, `ibge-regioes`, `ibge-municipios`, `senado-processos` e `senado-votacoes` ([issue #168](https://github.com/rikemorais/todeolhoaibr/issues/168)).
- Bronze para os **agregados do IBGE** (API v3, JSON aninhado): `ibge-ipca`, `ibge-inpc`, `ibge-pib-municipios`, `ibge-pib-trimestral`, `ibge-pnad-desocupacao` e `ibge-populacao-municipios`. Um `DecodificadorIbgeAgregados` explode a série aninhada em uma linha por (variável, localidade, período) ([issue #168](https://github.com/rikemorais/todeolhoaibr/issues/168)).
- Bronze para `senado-parlamentares` (array aninhado fundo no JSON do Senado); o `DecodificadorJson` passou a aceitar um **caminho** de chaves até o array ([issue #168](https://github.com/rikemorais/todeolhoaibr/issues/168)).
- Bronze para `senado-processos-detalhes` (um JSON por processo, ~163k arquivos): um decodificador **agrega** todos os processos de um ano em uma tabela (uma linha por processo), em vez de gerar um Parquet por arquivo. A `TarefaDeBronze` passou a decodificar as partes de um recurso em bloco, permitindo essa agregação ([issue #177](https://github.com/rikemorais/todeolhoaibr/issues/177)).
- O decodificador ZIP passou a aceitar também arquivos `.txt` (os anos antigos do TSE usam `.txt` no lugar de `.csv`, mesmo formato `;`/Latin-1) ([issue #168](https://github.com/rikemorais/todeolhoaibr/issues/168)).
- **Refino em fluxo de ponta a ponta** para arquivos grandes ([issue #171](https://github.com/rikemorais/todeolhoaibr/issues/171)): o part do raw é baixado do S3 **direto para um arquivo temporário** (novo `ArmazenamentoRaw::ler_fluxo`), o ZIP é lido do disco, cada CSV é decodificado em fluxo (Latin-1→UTF-8 on the fly) e o Parquet é escrito direto em arquivo — sem carregar o ZIP (de >1 GB), o CSV nem o Parquet inteiros na memória. Com isso entram no bronze **`tse-prestacao-contas`** e **`tse-votacao-secao`** (CSVs consolidados de vários GB), e `tse-candidaturas`/`tse-bens` usam o mesmo caminho.

### Corrigido

- O console SQL lista **todas** as tabelas, recriando as views a cada carregamento (fontes refinadas depois que o serviço subiu passam a aparecer sem reiniciar) e tolerando a view de uma fonte sem dados num período.

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
