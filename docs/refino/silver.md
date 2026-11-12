# Camada silver — contrato inicial

**Estado:** migração SQL/DataFusion concluída — todas as 29 consultas SQL estão ativas e as transformações Arrow manuais vivem somente em `#[cfg(test)]` para paridade ([issue #244](https://github.com/rikemorais/todeolhoaibr/issues/244), PR #306). O binário de produção não compila mais o código de transformação Arrow. A arquitetura aceita está no [ADR 0020](../adr/0020-contrato-da-camada-silver.md); responsabilidades das camadas estão no [ADR 0015](../adr/0015-arquitetura-de-dados-em-medalhao.md).

Silver lê somente o bronze vigente, aplica contratos por tabela e publica Parquet tipado no bucket `silver`. O contrato deve declarar grão, chave, tipos, nulos, duplicatas, qualidade e procedência. Chaves nesta matriz são **candidatas**: sua validação depende de amostras reais de cada tabela e de anos antigos/recentes. Identificadores, inclusive códigos IBGE, são texto; datas e valores devem ter tipos próprios. Nenhuma fonte sem município receberá um município inventado.

## Contribuir com uma transformação SQL

Contratos SQL ficam versionados em `crates/refino/sql/silver/` e entram no binário com `include_str!`. A consulta lê a tabela `bronze`, preparada somente com as partes do manifesto fixado. Além das colunas Bronze, há quatro colunas internas de linhagem: `_tdoa_origem_manifesto`, `_tdoa_origem_parte`, `_tdoa_origem_url` (nome/tabela da parte) e `_tdoa_origem_ordinal`. A URL permite que contratos selecionem uma tabela específica dentro de um recurso ZIP, como a parte consolidada `BRASIL` do TSE. Para contratos que preservam colunas de origem dinâmicas, o executor também pode fornecer `_tdoa_json_original`, serializado pelo adaptador; os casts, validações e projeções continuam no SQL. A consulta deve projetar explicitamente as colunas Silver definidas pelo schema do contrato. Valores externos nunca são interpolados no SQL; em `senado_processos.sql`, o ano é validado como número no caminho do recurso antes de ser inserido na consulta; detalhes usam a coluna Bronze `ano`.

O adaptador `TarefaDeSilverSql` associa a consulta embutida, nome/versionamento da regra e schema de saída ao publicador comum. A execução transmite batches para Parquet temporário; não use coleta integral do resultado. O limite inicial é 512 MiB de memória, 2 GiB de spill, 100 GiB de entrada, uma partição por consulta e batches de 8.192 linhas. Esses limites são configuráveis por tarefa e devem ser medidos antes de ampliar uma família de fontes.

O DataFusion 55.2 usado pelo projeto não inclui funções escalares centrais para extrair valores de JSON textual. O executor registra `tdoa_json_get(json, chave)` para leitura de campos escalares/objetos e `tdoa_json_expand_votos(json)` para normalizar as listas atuais e legadas do Senado em uma lista SQL que `UNNEST` expande. São funções genéricas de compatibilidade; aliases, validações, projeções e relacionamento permanecem declarados nos arquivos SQL. `UNNEST` descarta listas nulas ou vazias, que é a semântica desejada para votos: uma sessão sem voto continua na tabela de sessões e não cria linha filha.

O smoke e benchmark sintético reproduzível do executor está em [`benchmark-datafusion.md`](benchmark-datafusion.md). Ele mede o pipeline SQL local com schema e linhagem Bronze e registra claramente quando os dados são sintéticos; a validação semântica com arquivos reais continua nos issues de migração por fonte.

### Estrutura de arquivos

```
crates/refino/
├── sql/silver/              # 29 consultas SQL versionadas
│   ├── bcb_sgs.sql
│   ├── camara_deputados.sql
│   ├── ...
│   └── tse_votacao_secao.sql
└── src/silver/
    ├── mod.rs               # re-exports e schemas
    ├── publicacao.rs         # fluxo comum de publicação
    ├── tarefa_sql.rs         # testes do executor SQL
    ├── tarefa_bcb.rs         # adaptador BCB: SQL, schema, regra
    ├── tarefa_camara.rs      # adaptador Câmara partidos
    ├── ...                   # um tarefa_*.rs por família
    ├── bcb.rs                # transformação Arrow (#[cfg(test)])
    ├── camara_deputados.rs   # transformação Arrow (#[cfg(test)])
    └── ...                   # um <fonte>.rs de paridade por família
```

Cada contrato tem dois arquivos: a consulta SQL em `sql/silver/` e o adaptador Rust em `src/silver/tarefa_*.rs`. O adaptador declara `const REGRA`, o `include_str!` do SQL, o schema Arrow de saída e chama `publicar_transformacao_sql` do módulo `publicacao`. As transformações Arrow manuais em `src/silver/<fonte>.rs` existem somente sob `#[cfg(test)]` e servem de oráculo para fixtures de paridade.

### Convenções SQL

1. **Comentário inicial**: a primeira linha é `-- Contrato <nome-do-contrato>.` seguido de uma frase descritiva.
2. **Tabela de entrada**: sempre `FROM bronze`; o executor registra o Parquet Bronze como tabela virtual `bronze`.
3. **Projeção explícita**: a consulta deve projetar todas as colunas do schema Silver, nunca `SELECT *` na saída final.
4. **Linhagem obrigatória**: toda consulta projeta `_tdoa_origem_manifesto`, `_tdoa_origem_parte` e `_tdoa_origem_ordinal`; o publicador rejeita a saída sem essas três colunas.
5. **Validação de chave**: usar CTE com `COUNT(*) OVER (PARTITION BY ...)` ou agregação escalar para verificar unicidade e falhar com `tdoa_assert_true` quando a chave duplica.
6. **Sem interpolação externa**: valores da entrada do usuário nunca são interpolados no SQL. O `{{ANO_RECURSO}}` é substituído pelo adaptador Rust após validar o ano como número inteiro no caminho do recurso.
7. **Nomes de coluna**: colunas Silver usam `snake_case`; colunas Bronze herdadas mantêm o nome original entre aspas duplas (`"codigoSessaoVotacao"`).
8. **Ordenação**: terminar com `ORDER BY _tdoa_origem_manifesto, _tdoa_origem_parte, _tdoa_origem_ordinal` para saída determinística.

### Tipos e sentinelas

| Domínio | Tipo Arrow/Parquet | Convenção |
| --- | --- | --- |
| Identificadores, códigos, UFs | `Utf8` (texto) | Nunca converter para inteiro; zeros à esquerda são significativos |
| Ano | `UInt16` | Validar como inteiro positivo de quatro dígitos |
| Datas | `Date32` | Converter com `to_date` ou `TRY_CAST`; sentinelas e formatos inválidos viram nulo sem descartar a linha |
| Valores monetários | `Decimal128(18,2)` ou `(18,6)` | Aritmética decimal exata; BCB usa ponto, Senado e TSE usam vírgula brasileira |
| Contadores | `UInt32` ou `UInt64` | Sequências e ordens que cabem no tipo |
| Idade | `UInt8` | Sentinelas (−1, 999) viram nulo |
| JSON preservado | `Utf8` | `registro_original` ou `_tdoa_json_original` como texto JSON compacto |

Sentinelas textuais da origem (`-1`, `-4`, `#NULO#`, `#NE#`, strings vazias) nunca são interpretadas como valor; o contrato as converte em nulo e preserva o texto original em `registro_original`.

### Placeholders e UDFs do executor

**Placeholders** — expandidos pelo adaptador Rust antes de enviar ao DataFusion:

| Placeholder | Expansão |
| --- | --- |
| `{{COLUNAS_BRONZE}}` | Todas as colunas do Parquet Bronze, qualificadas com `origem."nome"` |
| `{{EXPR_OPCIONAL:NOME}}` | `"NOME"` se a coluna existir no Bronze; `CAST(NULL AS VARCHAR)` caso contrário |
| `{{OPCIONAL:NOME}}` | Idem, mas acrescenta `AS "NOME"` na projeção |
| `{{ANO_RECURSO}}` | Literal inteiro do ano extraído do caminho do recurso (validado no Rust) |

**UDFs registradas** — disponíveis em qualquer consulta SQL Silver:

| Função | Assinatura | Uso |
| --- | --- | --- |
| `tdoa_assert_true(expr)` | `Boolean → Boolean` | Falha a consulta se `expr` for falso ou nulo; usada para validação de chaves |
| `tdoa_assert_true_named(expr, nome)` | `(Boolean, Utf8) → Boolean` | Idem, com nome da asserção na mensagem de erro |
| `tdoa_json_get(json, chave)` | `(Utf8, Utf8) → Utf8` | Extrai campo escalar ou objeto de JSON textual |
| `tdoa_json_expand_votos(json)` | `Utf8 → List<Utf8>` | Normaliza arrays de votos do Senado para `UNNEST` |

### Versionamento da regra

Cada adaptador declara uma constante `REGRA` com o formato `<fonte>-<descrição>-v<N>`:

- `v1`: primeira versão do contrato (Arrow manual ou SQL inicial).
- `v2`: reprocessamento SQL que substitui a saída Arrow anterior; a reconciliação republica uma vez.
- `v3+`: correção de contrato (ex.: `senado-despesa-ceaps-v4` corrigiu vírgula brasileira histórica).

A versão Silver publicada é `<id_execução_bronze>|regra=<REGRA>`. O publicador compara essa string antes de executar; se já existe, reaproveia sem reprocessar (idempotência por versão Bronze + regra).

Incrementar a versão força reprocessamento de todos os recursos da família. Fazer isso somente quando a semântica da saída muda.

### Fixtures e testes

Os 24 contratos são cobertos por testes em `crates/refino/src/silver/tarefa_sql.rs` e nos respectivos `tarefa_*.rs`. Os testes usam fixtures Parquet em memória com `ArmazenamentoEmMemoria`:

1. **Fixtures de paridade**: comparam a saída SQL com a saída da transformação Arrow manual (`#[cfg(test)]`). Garantem que a migração não alterou os dados publicados.
2. **Fixtures de borda**: testam datas inválidas, decimais fora de escala, sentinelas, JSON nulo, chaves duplicadas, arrays vazios e colunas ausentes.
3. **Teste de inventário SQL** (PR #307): verifica bidirecionalmente que todo `.sql` em `sql/silver/` tem `include_str!` correspondente e vice-versa — sem arquivos órfãos nem contratos fantasma.
4. **Testes de linhagem**: confirmam que `_tdoa_origem_manifesto`, `_tdoa_origem_parte` e `_tdoa_origem_ordinal` estão presentes e corretos na saída.

Executar os testes localmente:

```bash
cd crates/refino
cargo test --lib silver
```

### Execução local

```bash
# Uma família específica
todeolho refinar silver bcb
todeolho refinar silver senado-votacoes
todeolho refinar silver tse-candidaturas

# Todas as famílias (exceto tse-votacao-secao pelo volume)
todeolho refinar silver --todas

# Reconciliação periódica (intervalo em segundos)
todeolho refinar silver --todas --intervalo 3600

# Filtros para votação por seção
todeolho refinar silver tse-votacao-secao --filtro ano=2024 --filtro uf=AC,RO
```

O executor usa os limites padrão (512 MiB de memória, 2 GiB de spill). Para alterar por tarefa, configurar `ConfiguracaoExecutorSql` no adaptador. A votação por seção usa 256 MiB de pool e 8 GiB de spill pelo volume de dados.

### Processo de revisão de um novo contrato

1. **Validar chave no Bronze**: medir unicidade com DuckDB sobre os Parquets locais; registrar evidência na seção de cobertura deste documento.
2. **Criar o SQL**: arquivo em `crates/refino/sql/silver/<fonte>.sql` seguindo as convenções acima. Usar um contrato existente como modelo (ex.: `bcb_sgs.sql` para fontes simples, `tse_candidaturas.sql` para fontes com placeholders e chave composta).
3. **Criar o adaptador**: `tarefa_<fonte>.rs` com `const REGRA`, `include_str!`, schema Arrow e chamada ao publicador comum.
4. **Registrar no catálogo**: adicionar a família em `mod.rs`, na CLI e em `--todas` (se o volume permitir).
5. **Testes**: fixture de borda e, se houver transformação Arrow anterior, fixture de paridade.
6. **Smoke local**: executar contra o Bronze da máquina e conferir contagem, chaves e idempotência na segunda passada.
7. **Documentar**: atualizar a cobertura neste arquivo e na matriz de chaves.

### Exceções Rust mantidas fora do SQL

As transformações Arrow manuais estão todas sob `#[cfg(test)]` e não compilam no binário de produção. Não há exceções Rust em produção — toda a lógica de negócio Silver executa via SQL/DataFusion pelo publicador comum `publicar_transformacao_sql`.

O código Rust dos adaptadores (`tarefa_*.rs`) contém apenas:

- Declaração do schema Arrow de saída.
- Constante `REGRA` com a versão.
- `include_str!` do SQL.
- Expansão de `{{ANO_RECURSO}}` quando necessário (após validação do ano como inteiro).
- Chamada ao publicador comum (`publicacao::publicar_transformacao_sql`).
- Configuração de limites de memória/spill quando diverge do padrão.

As UDFs Rust (`tdoa_assert_true`, `tdoa_json_get`, `tdoa_json_expand_votos`) são funções genéricas do executor, não transformações por contrato.

### Inventário dos 25 contratos SQL

| # | Arquivo SQL | Adaptador Rust | Regra | Família |
| --- | --- | --- | --- | --- |
| 1 | `bcb_sgs.sql` | `tarefa_bcb.rs` | `bcb-sgs-v2` | BCB |
| 2 | `ibge_agregados.sql` | `tarefa_ibge.rs` | `ibge-agregado-v2` | IBGE |
| 3 | `ibge_renda_per_capita.sql` | `tarefa_ibge_renda_per_capita.rs` | `ibge-renda-per-capita-v1` | IBGE |
| 4 | `senado_parlamentares.sql` | `tarefa_senado.rs` | `senado-parlamentar-v2` | Senado |
| 5 | `senado_votacoes.sql` | `tarefa_senado_votacoes.rs` | `senado-votacao-v2` | Senado |
| 6 | `senado_votos.sql` | `tarefa_senado_votacoes.rs` | `senado-voto-nominal-v2` | Senado |
| 7 | `senado_despesas.sql` | `tarefa_senado_despesas.rs` | `senado-despesa-ceaps-v4` | Senado |
| 8 | `senado_processos.sql` | `tarefa_senado_processos.rs` | `senado-processo-v2` | Senado |
| 9 | `senado_processos_detalhes.sql` | `tarefa_senado_processos_detalhes.rs` | `senado-processo-detalhe-v2` | Senado |
| 10 | `camara_partidos.sql` | `tarefa_camara.rs` | `camara-partido-v2` | Câmara |
| 11 | `camara_legislaturas.sql` | `tarefa_camara_legislaturas.rs` | `camara-legislatura-periodo-v2` | Câmara |
| 12 | `camara_deputados.sql` | `tarefa_camara_deputados.rs` | `camara-deputado-exercicio-v2` | Câmara |
| 13 | `camara_despesas.sql` | `tarefa_camara_despesas.rs` | `camara-despesa-ceap-v3` | Câmara |
| 14 | `camara_tramitacoes.sql` | `tarefa_camara_tramitacoes.rs` | `camara-tramitacao-evento-v2` | Câmara |
| 15 | `camara_votacoes_sessoes.sql` | `tarefa_camara_votacoes.rs` | `camara-votacoes-v2` | Câmara |
| 16 | `camara_votacoes_votos.sql` | `tarefa_camara_votacoes.rs` | `camara-votacoes-votos-v3` | Câmara |
| 17 | `camara_votacoes_proposicoes.sql` | `tarefa_camara_votacoes.rs` | `camara-proposicoes-v2` | Câmara |
| 18 | `camara_votacoes_proposicoes_autores.sql` | `tarefa_camara_votacoes.rs` | `camara-proposicoes-autores-v2` | Câmara |
| 19 | `camara_votacoes_proposicoes_relacionadas.sql` | `tarefa_camara_votacoes.rs` | `camara-votacoes-proposicoes-v2` | Câmara |
| 20 | `camara_votacoes_objetos.sql` | `tarefa_camara_votacoes.rs` | `camara-votacoes-objetos-v2` | Câmara |
| 21 | `camara_votacoes_orientacoes.sql` | `tarefa_camara_votacoes.rs` | `camara-votacoes-orientacoes-v2` | Câmara |
| 22 | `tse_candidaturas.sql` | `tarefa_tse_candidaturas.rs` | `tse-candidatura-registro-v2` | TSE |
| 23 | `tse_bens.sql` | `tarefa_tse_bens.rs` | `tse-bem-candidato-v2` | TSE |
| 24 | `tse_despesas_pagas.sql` | `tarefa_tse_despesas_pagas.rs` | `tse-despesa-paga-2024-v2` | TSE |
| 25 | `tse_votacao_secao.sql` | `tarefa_tse_votacao_secao.rs` | `tse-votacao-secao-v2` | TSE |

O teste de inventário (`nenhum_sql_silver_orfao_nem_contrato_sem_consulta`) garante em CI que esta lista permanece sincronizada com o código.

## Contratos eleitorais SQL/DataFusion

As quatro famílias Silver já existentes do TSE usam SQL versionado executado pelo publicador comum: candidaturas, bens, votação por seção e despesas pagas. O executor expande placeholders de campos opcionais e aliases são combinados explicitamente nas consultas, preservando layouts históricos. Os parâmetros anuais vêm de recursos validados; a votação por seção aceita somente ano de quatro dígitos e UF de duas letras maiúsculas. A origem integral continua em `registro_original`; as três referências Bronze são projetadas na saída e exigidas pelo publicador. A votação por seção permanece fora de `silver --todas` por seu volume local de 16,4 GB. As regras v2 provocam uma reconciliação dos Parquets publicados com Arrow.

## Cobertura planejada do catálogo bronze

O [catálogo](../../crates/refino/src/catalogo.rs) tem 26 entradas. Uma entrada ZIP pode produzir várias tabelas internas; cada uma precisará de contrato próprio. `recurso` e `versão bronze` fazem parte da procedência, mas não substituem a chave de negócio.

A [especificação de contratos da fase 0](contratos-silver-fase0.md) detalha tipos,
nulos, normalização, deduplicação e tabelas internas para cada família. Esta
matriz resume o grão e a evidência de chave; “candidata” não autoriza descartar
linhas nem ativar uma relação sem validação do recorte.

| Entrada bronze | Família | Grão candidato da silver | Chave candidata / validação necessária |
| --- | --- | --- | --- |
| `camara-despesas` | Legislativo | lançamento CEAP | `ideDocumento` é candidato, mas não único; preservar todas as linhas até definir chave |
| `camara-votacoes` | Legislativo | registro de um dos sete conjuntos anuais | contrato separado para `proposicoes`, `proposicoesAutores`, `votacoes`, `votacoesVotos`, `votacoesObjetos`, `votacoesProposicoes` e `votacoesOrientacoes`; chaves de 2024 abaixo |
| `camara-tramitacoes` | Legislativo | evento de tramitação | não há chave de negócio única comprovada; preservar repetições e usar proveniência do recurso/snapshot |
| `camara-deputados` | Legislativo | deputado em exercício na versão Bronze vigente | `id` oficial (513/513 distintos na amostra local; cada versão é uma fotografia) |
| `camara-partidos` | Legislativo | partido no cadastro observado | `id` oficial (21/21 distintos na amostra local; não prova unicidade histórica) |
| `camara-legislaturas` | Legislativo | período de atividade parlamentar | `id` sequencial (57/57 distintos na amostra local) |
| `senado-despesas` | Legislativo | lançamento CEAPS | `COD_DOCUMENTO` candidato, único quando presente na amostra; preservar linhas sem código |
| `senado-votacoes` | Legislativo | uma sessão de votação nominal | `codigoSessaoVotacao` (3.652/3.652 distintos no recorte local consultado); sessão conserva JSON original, e votos são publicados também em tabela filha com ponteiro próprio |
| `senado-parlamentares` | Legislativo | parlamentar em legislatura | `IdentificacaoParlamentar.CodigoParlamentar` e `legislatura` no bronze local |
| `senado-processos` | Legislativo | um processo na listagem anual | `id` textual único no recurso; `ano` UInt16 do caminho |
| `senado-processos-detalhes` | Legislativo | um detalhe por processo em cada recurso anual | `id` textual; `ano` UInt16; demais campos preservados em `processo_original` |
| `tse-candidaturas` | Eleitoral | candidatura por eleição | ano, eleição, UF, unidade eleitoral, cargo, número e sequência do candidato; [validação histórica abaixo](#candidaturas-e-bens-do-tse-no-bronze-local-2026-11-05) |
| `tse-bens` | Eleitoral | bem declarado | ano, eleição, UF, unidade eleitoral, sequência do candidato e ordem do bem; alias da ordem muda por ano |
| `tse-prestacao-contas` | Eleitoral | lançamento por tabela interna | contrato por conjunto e ano; sequências de 2024 colidem no consolidado, conforme [validação abaixo](#prestacao-de-contas-do-tse-em-2024); sondas de chaves para as seis famílias de 2014 e as duas famílias partidárias de 2016 não encontraram chave completa e única; ver [medições e inventário 2014–2022](pesquisa-tse-2014-2022.md), [fontes oficiais 2016–2022](pesquisa-tse-2016-2022.md) e [pesquisa sobre `_sup`](pesquisa-tse-2014-sup.md) |
| `tse-votacao-secao` | Eleitoral | voto agregado por seção e votável | ano, eleição, UF, turno, município, zona, seção, cargo e votável; [recortes validados abaixo](#votacao-por-secao-do-tse-no-bronze-local-2026-11-05) |
| `bcb` | Série | observação SGS | série (do recurso) e `data`; `valor` decimal; data no formato `DD/MM/AAAA` no fixture |
| `ipeadata` | Série | observação OData | `SERCODIGO`, `TERCODIGO` e `VALDATA` candidatos; `NIVNOME` distingue Brasil, estados, municípios e regiões |
| `ibge-estados` | Território | UF | código IBGE e vigência |
| `ibge-regioes` | Território | região | código IBGE e vigência |
| `ibge-municipios` | Território | município | código IBGE e vigência |
| `ibge-ipca` | Agregado IBGE | variável × localidade × período | variável, classificações, nível, localidade e período; validar classificações |
| `ibge-inpc` | Agregado IBGE | variável × localidade × período | idem |
| `ibge-pib-municipios` | Agregado IBGE | variável × localidade × período | idem; município somente se nível N6 |
| `ibge-pib-trimestral` | Agregado IBGE | variável × localidade × período | idem |
| `ibge-pnad-desocupacao` | Agregado IBGE | variável × localidade × período | idem |
| `ibge-populacao-municipios` | Agregado IBGE | variável × localidade × período | idem; município somente se nível N6 |

## Contrato mínimo por tabela

1. Nome estável, tabela bronze de origem, grão e chave de negócio, incluindo escopo temporal.
2. Colunas de saída, tipos Arrow/Parquet, nulabilidade, unidades e conversão de sentinelas.
3. Regra de deduplicação determinística e tratamento de correções da origem.
4. Política para inválidos: motivo, contagem, referência à linha de origem e limiar de falha.
5. Versão do contrato/regra e linhagem até manifesto, recurso e tabela bronze.

### Qualidade e publicação

O [ADR 0020](../adr/0020-contrato-da-camada-silver.md#politica-de-qualidade-e-falha-parcial)
fixa limiar zero por padrão para falha de campo obrigatório, conversão
obrigatória, conflito de chave e perda de linhagem. Conversão opcional para
nulo exige valor original preservado e contagem por motivo. Duplicata integral
é removida apenas por regra específica comprovada; famílias sem identidade
validada preservam todas as ocorrências com manifesto, parte e ordinal Bronze.
Uma falha mantém o ponteiro Silver anterior. Diagnóstico recuperável por linha
e métricas de rejeição são parte do contrato alvo; o publicador atual ainda não
os emite de forma estruturada, então não ativar um contrato que dependa dessa
quarentena antes de implementá-la.

A referência `(manifesto, parte, ordinal)` consta da linha Silver publicada,
além do manifesto da execução. O leitor Bronze a fornece ao executor SQL; os
contratos ativados a projetam e o publicador rejeita referência ausente ou
nula. O teste de linhas Bronze idênticas confirma ordinais distintos. Votos
do Senado acrescentam `ordem_voto` para distinguir filhos da mesma linha.
Sessões e votos têm ponteiros separados; uma falha em votos não apaga a
publicação anterior de sessões. O CAS distribuído (`gravar_condicional` no `ArmazenamentoRaw`), a validação da
versão Bronze no leitor Silver (`SnapshotBronze::ainda_vigente`) e os testes de
condição de corrida foram entregues na
[issue #300](https://github.com/rikemorais/todeolhoaibr/issues/300) (v0.53.0).

### Referência técnica da linha bronze

O leitor [`LeitorDeParteBronze`](../../crates/refino/src/linhagem.rs) percorre uma parte Parquet em lotes, sem projeção ou filtro, e associa cada registro à chave do manifesto bronze, à chave da parte, ao nome da tabela de origem e ao ordinal da linha na parte (começando em 1). O ordinal continua entre lotes e não depende do tamanho do lote de leitura. Assim, duas linhas iguais no mesmo Parquet ainda têm referências distintas, e variantes com a mesma sequência oficial permanecem rastreáveis.

O [`SnapshotBronze`](../../crates/refino/src/snapshot_bronze.rs) lê o ponteiro e o manifesto vigentes, confere fonte/recurso e permite baixar somente partes listadas no manifesto. O download é em fluxo para arquivo temporário; tamanho e SHA-256 são verificados antes de abrir o leitor em lotes. Ponteiro ou manifesto inválido causam erro; ausência de ponteiro ou manifesto válido sem partes significa que não há dados Bronze a transformar.

Esse snapshot fixa a versão lida no início. A tarefa Silver ainda deverá conferir o ponteiro bronze antes de publicar sua própria versão, para não publicar resultado de uma entrada que já foi substituída. A referência da linha vale somente para a versão bronze selecionada e não substitui a chave de negócio. Nenhuma tabela Silver é publicada por esse leitor.

O [`EscritorParquetTipado`](../../crates/refino/src/escritor.rs) recebe o schema Arrow do contrato Silver e grava lotes `RecordBatch` em Parquet, inclusive para um arquivo temporário. Ele exige correspondência exata de nomes, tipos e nulabilidade em cada lote; data e decimal permanecem tipados. O teste de ida e volta cobre dois lotes, nulo e rejeição de escala decimal divergente.

O contrato `bcb.sgs.observacao.v1` é definido em [`bcb_sgs.sql`](../../crates/refino/sql/silver/bcb_sgs.sql): `data` em `Date32` e `valor` em `Decimal128(18,6)`, sem conversão por `f64`. A consulta exige data `DD/MM/AAAA` e valor decimal com ponto e até seis casas; dados inválidos ou nulos falham antes de publicar. O adaptador [`TarefaDeSilverBcb`](../../crates/refino/src/silver/tarefa_bcb.rs) usa `TarefaDeSilverSql` e o publicador comum, que mantém manifesto, ponteiro, idempotência e revalidação do snapshot Bronze. A regra `bcb-sgs-v2` reprocessa uma vez o que foi publicado pelo transformador Arrow anterior. A nulabilidade do schema publicado permanece a do contrato; nulos em campos obrigatórios são rejeitados na borda Arrow/Parquet. A CLI aceita `todeolho refinar silver bcb` e `todeolho refinar silver --todas --intervalo 3600`. A execução Silver aparece no painel com o selo azul.

O contrato `ibge.renda_per_capita.observacao.v1` está em [`ibge_renda_per_capita.sql`](../../crates/refino/sql/silver/ibge_renda_per_capita.sql). Mapeia as colunas codificadas do SIDRA `/values` (`D1C`→`municipio_id`, `D2C`→`variavel_id`, `D3C`→`ano_censo`, `V`→`valor_original`/`valor_decimal`) e filtra o cabeçalho descritivo (`NC <> 'Nível Territorial (Código)'`) e sentinelas de supressão (`..`, `-`, `X`). A regra `ibge-renda-per-capita-v1` cobre os Censos 2010 (tabela 3578) e 2022 (tabela 10295). `TarefaDeSilverIbgeRendaPerCapita` publica manifesto e ponteiro com idempotência por versão Bronze/regra. A CLI rota `ibge-renda-per-capita` antes do catch-all `ibge-*` para usar o adaptador próprio.

O contrato `ibge.agregado.observacao.v1` está em [`ibge_agregados.sql`](../../crates/refino/sql/silver/ibge_agregados.sql). Mantém IDs, classificações e período como texto, preserva o valor recebido em `valor_original` e acrescenta `valor_decimal` (`Decimal128(38,9)`) quando o conteúdo corresponde a decimal simples com ponto. Valor nulo, sentinela textual ou valor acima da escala permitida recebe decimal nulo, sem descartar a linha nem classificar sentinelas sem metadados da tabela. `TarefaDeSilverIbge` publica manifesto e ponteiro com idempotência por versão Bronze/regra e verifica que o ponteiro Bronze não mudou durante a transformação. A regra `ibge-agregado-v2` reprocessa uma vez os recursos que foram publicados pelo transformador Arrow anterior. A CLI oferece as seis fontes agregadas com contrato: `ibge-ipca`, `ibge-inpc`, `ibge-pib-municipios`, `ibge-pib-trimestral`, `ibge-pnad-desocupacao` e `ibge-populacao-municipios`. `ibge-estados`, `ibge-regioes` e `ibge-municipios` ainda não têm contrato Silver e ficam fora de `--todas`. A reconciliação percorre as fontes suportadas; passadas vazias não são registradas no painel. Fixtures comparam o Parquet SQL ao transformador Arrow anterior, mantido somente em builds de teste.

### Paridade e smoke SQL/DataFusion (2026-11-06)

As fixtures de publicação comparam os `RecordBatch` Silver da consulta com a implementação Arrow anterior. Cobrem datas e decimais BCB válidos, datas inválidas/nulas, precisão excedida, chaves IBGE que incluem classificação, códigos com zero à esquerda, sentinelas textuais e valores nulos. Os testes confirmam chave única nos casos válidos e ausência de ponteiro quando a consulta BCB rejeita dados inválidos.

O smoke foi executado com o binário local no perfil dev e o Bronze/VersityGW desta máquina. As onze séries BCB publicaram 5.125 linhas em 46.676 bytes de Parquet; a execução levou aproximadamente 1 s e a chave (`recurso`, `data`) não teve duplicatas nem datas nulas. O IPCA publicou 562 linhas; uma passada das seis famílias IBGE publicou/reutilizou no total 247.923 linhas. Consulta de somente leitura no DuckDB encontrou zero linhas em chaves repetidas nos seis recortes, usando (`variavel_id`, `classificacoes`, `nivel_id`, `localidade_id`, `periodo`).

| Fonte IBGE | Linhas publicadas | Linhas em chaves repetidas |
| --- | ---: | ---: |
| IPCA | 562 | 0 |
| INPC | 571 | 0 |
| PIB municipal | 122.540 | 0 |
| PIB trimestral | 122 | 0 |
| PNAD desocupação | 1.566 | 0 |
| População municipal | 122.562 | 0 |

O executor usou os limites padrão de 512 MiB de memória e 2 GiB de spill; o smoke real não amostrou RSS nem spill. Como referência operacional do engine, o benchmark sintético em [`benchmark-datafusion.md`](benchmark-datafusion.md) processou 10 milhões de linhas com orçamento de 64 MiB, RSS amostrado de 187.551.744 bytes e 160.236.672 bytes de spill em 27.503 ms; esses números não representam as fontes BCB/IBGE. A primeira compilação `--release` limpa da imagem levou 8 min 28 s, incluindo compilação inicial das dependências. As implementações manuais antigas ficam sob `cfg(test)` para paridade; em runtime, as duas regras usam 33 linhas de SQL versionado, além dos schemas e adaptadores Rust.

O contrato `senado.parlamentar.legislatura.v1` extrai `CodigoParlamentar` com `tdoa_json_get` no contrato SQL [`senado_parlamentares.sql`](../../crates/refino/sql/silver/senado_parlamentares.sql), tipa `legislatura` como `UInt16` a partir do recurso `parlamentares/legislatura=NN` e preserva os textos originais de identificação e mandato. O código permanece texto, inclusive quando a API o entrega como número, para não perder zeros à esquerda nos IDs textuais. A chave candidata (`CodigoParlamentar`, `legislatura`) foi validada no Bronze local; o mandato aninhado não é explodido. A regra `senado-parlamentar-v2` reprocessa os snapshots transformados pela versão Arrow. Smoke local em 2026-11-07 publicou as nove legislaturas vigentes no Bronze (49–57), e a segunda reconciliação não alterou nenhuma delas. A [OpenAPI oficial do Senado](https://legis.senado.leg.br/dadosabertos/v3/api-docs) documenta a listagem por legislatura. A CLI oferece `todeolho refinar silver senado-parlamentares`; a fonte também entra em `--todas` e na reconciliação periódica.

O contrato `senado.processo.v1` publica uma linha por matéria em cada listagem anual, tipa `id` como texto e `ano` como `UInt16`, e mantém todos os campos da API em `processo_original` JSON. A transformação e a validação de IDs únicos estão em `senado_processos.sql`; o ano validado do recurso anual é fornecido como parâmetro SQL. Uma coluna interna conserva dinamicamente todos os campos e valores JSON da origem, inclusive campos novos, antes de sua projeção no contrato. A listagem anual é uma fotografia do catálogo consultado naquele ano, não um histórico de versões da matéria. IDs foram únicos e não vazios nos snapshots locais de 2010 (2.965 linhas), 2023 (5.432) e 2024 (4.947); essa validação é por recurso anual. A regra v2 reprocessa uma vez os snapshots existentes. Uma lista JSON sem registros não gera parte Bronze e, portanto, não tenta criar um Parquet Silver de schema indefinido. A CLI oferece `todeolho refinar silver senado-processos`; a fonte entra em `--todas` e na reconciliação periódica.

O contrato `senado.processo.detalhe.v1` publica uma linha por processo no recurso anual, com `id` como texto, `ano` como `UInt16` e os campos Bronze preservados em `processo_original` JSON. A consulta `senado_processos_detalhes.sql` valida ano e unicidade do ID sem explodir despachos, classificações ou outros objetos aninhados; `_tdoa_json_original` inclui dinamicamente todas as colunas Bronze. A regra v2 reprocessa uma vez os snapshots existentes. No Bronze local consultado, 163.156 linhas tinham 163.156 IDs distintos e anos válidos. A CLI oferece `todeolho refinar silver senado-processos-detalhes`; a fonte entra em `--todas` e na reconciliação periódica.

O leitor Silver também aceita partes Bronze legadas cujo Parquet tem schema raiz vazio e nenhuma linha: elas produzem uma parte Silver vazia usando o schema declarado pelo contrato. Um arquivo sem colunas que declare linhas continua sendo erro.

O contrato `senado.votacao.sessao.v1` é executado por [`senado_votacoes.sql`](../../crates/refino/sql/silver/senado_votacoes.sql): valida `codigoSessaoVotacao` por recurso anual e preserva cada registro do Bronze, incluindo `votos` como JSON textual aninhado. A chave teve 3.652 valores distintos em 3.652 linhas no recorte local consultado; os demais anos disponíveis foram cobertos pelo smoke local descrito abaixo. O contrato complementar `senado.voto.nominal.v1`, em [`senado_votos.sql`](../../crates/refino/sql/silver/senado_votos.sql), publica uma linha por voto em `votos/ano=AAAA`, mantendo sessão, data, ordinal original, campos disponíveis do parlamentar, voto e objeto integral em `voto_original`. O SQL expande arrays com `UNNEST`; a UDF de compatibilidade aceita arrays e envelopes `votoParlamentar`/`VotoParlamentar`, inclusive sob `Votos`/`votos`, e voto único como objeto. A chave técnica é (`codigo_sessao_votacao`, `ordem_voto`), sem descartar ocorrências repetidas. Sessões com arrays vazios ou votos nulos continuam na tabela de sessões e não criam linhas filhas; identificadores parlamentares ausentes ou inválidos interrompem a publicação. A regra `senado-votacao-v2`/`senado-voto-nominal-v2` reprocessa as versões anteriores. Os testes comparam os lotes SQL ao transformador Arrow em fixtures atuais e legados, incluindo array vazio e JSON nulo, e validam a ordem dos votos e os vínculos com as sessões. Smoke local em 2026-11-07 publicou os 36 anos disponíveis no Bronze (1991–2026), com relações de sessão e voto para cada ano; a segunda reconciliação reportou `sem alterações na origem` para todos. A documentação oficial descreve os campos como tabelas relacionadas ([serviço de votações nominais](https://www12.senado.leg.br/dados-abertos/legislativo/plenario/votacoes-nominais/info/webservice-de-votacoes-nominais-por-data-da-sessao-plenaria)). A CLI `todeolho refinar silver senado-votacoes` publica as duas relações e segue incluída em `--todas` e na reconciliação periódica.

O contrato `camara.partido.v1` valida `id` como identificador não nulo e não vazio e preserva `id`, `sigla`, `nome` e `uri` como texto. O ID teve 21 valores distintos nas 21 linhas da amostra local descrita acima; isso é evidência apenas para o cadastro observado, não para histórico ou futuras versões. A projeção e a validação de unicidade agora ficam no contrato SQL `camara_partidos.sql`, executado pelo publicador DataFusion. A regra v2 reprocessa os snapshots existentes uma vez. A tarefa acompanha o ponteiro Bronze e é idempotente por versão e regra. A CLI oferece `todeolho refinar silver camara-partidos`; a fonte também entra em `--todas` e na reconciliação periódica.

O contrato `camara.legislatura.periodo.v1` tipa o ID sequencial como `UInt16`, as datas `dataInicio` e `dataFim` como `Date32` e o ano `dataEleicao` como `UInt16`; preserva `uri` como texto. Rejeita IDs inválidos ou duplicados, datas fora do formato ISO e intervalos invertidos. A transformação agora está no contrato SQL `camara_legislaturas.sql`, que usa `arrow_try_cast` para produzir os tipos unsigned do contrato; a regra v2 reprocessa os snapshots existentes uma vez. A documentação oficial descreve os períodos parlamentares como sequenciais desde a primeira legislatura e lista início, fim e ano eleitoral ([API de legislaturas](https://dadosabertos.camara.leg.br/api/v2/api-docs)). O conjunto observado teve 57 IDs distintos em 57 linhas. A CLI oferece `todeolho refinar silver camara-legislaturas`; a fonte participa de `--todas` e da reconciliação periódica.

O contrato `camara.deputado.exercicio.v1` trata cada versão Bronze como a fotografia vigente dos deputados em exercício, sem inferir histórico a partir de uma listagem sem intervalo temporal. Preserva `id`, URIs, nome, partido, UF, legislatura, foto e e-mail; identificadores e atributos categóricos permanecem texto. `id` deve ser numérico, obrigatório e único na fotografia. A transformação está no contrato SQL `camara_deputados.sql`, e a regra v2 reprocessa os snapshots existentes uma vez. A lista da Câmara teve 513 IDs distintos nas 513 linhas observadas. A [API oficial](https://dadosabertos.camara.leg.br/api/v2/api-docs) descreve o endpoint de deputados e os filtros temporais; a CLI oferece `todeolho refinar silver camara-deputados`, incluído em `--todas` e na reconciliação periódica.

Os sete contratos da fonte `camara-votacoes` são SQL versionado em `crates/refino/sql/silver/`. Validam a chave candidata de cada conjunto por recurso anual e projetam somente as colunas de origem, preservando todas as linhas e o schema Bronze (texto); diferenças de colunas entre recursos são acomodadas pela projeção montada a partir do schema observado. As regras usam v2; `votacoesVotos` usa v3 para reprocessar a publicação anterior. Não fazem deduplicação nem cruzamento entre tabelas. Em `votacoesVotos`, linhas com chave candidata incompleta são preservadas e não participam da verificação de duplicidade. No snapshot local de 2004 havia uma linha sem `deputado_id`; ela foi mantida na publicação. Os Parquets Silver são independentes por conjunto e ano; `--todas` percorre os recursos Bronze presentes, e a pré-checagem inclui snapshot e versão da regra. A CLI oferece `todeolho refinar silver camara-votacoes`. A matriz de unicidade disponível foi medida somente em 2024: os anos anteriores ainda precisam de smoke test contra Bronze vigente antes de serem considerados cobertos por evidência.

Na execução `senado-votacoes`, sessões e votos agora são tarefas distintas no
mesmo DAG, identificadas por `votacoes/ano=AAAA` e `votos/ano=AAAA`. Cada tabela
tem status e ponteiro próprios no mesmo registro de execução. Se uma falhar,
o resultado da outra permanece consultável e a reconciliação retoma apenas a
que estiver pendente.

O console DuckDB cria views `bronze_*` e `silver_*` a partir dos ponteiros vigentes e dos manifestos correspondentes. As views Silver incluem apenas Parquets publicados pela Silver; execuções antigas permanecem fora do glob.

## Evidência disponível no repositório

### Despesas da Câmara (CEAP)

O contrato `camara.despesa.ceap.v2` é executado por SQL/DataFusion (regra `camara-despesa-ceap-v3`) e publica uma linha por lançamento anual, tipa o ano do caminho como `UInt16`, datas de emissão/restituição como `Date32` e os quatro valores monetários como `Decimal128(18,6)`. A precisão de seis casas é necessária para preservar valores históricos de 2013–2015; no Bronze local, nenhuma linha supera 12 dígitos inteiros ou seis decimais. Identificadores permanecem texto. Campos ausentes ou em branco nesses campos opcionais viram nulos; valor preenchido fora do formato esperado interrompe a publicação. `registro_original` preserva todos os campos do Bronze, inclusive campos que ainda não têm projeção tipada. A consulta não deduplica: `ideDocumento` colide no conjunto histórico e não é chave única comprovada. A CLI `todeolho refinar silver camara-despesas` aceita um recurso anual, participa de `--todas` e da reconciliação Silver.

### Tramitações da Câmara

O contrato `camara.tramitacao.evento.v1` é executado por SQL/DataFusion (regra `camara-tramitacao-evento-v2`), publica cada linha do recurso anual e tipa `ano` como `UInt16`, `sequencia` como `UInt32` quando presente e `dataHora` como `Timestamp(Microsecond)`. Identificadores, códigos e números permanecem texto; `registro_original` retém todos os campos Bronze. Colunas opcionais ausentes em alguns anos são materializadas como nulas. A amostra local tinha 1.879.478 linhas: `idTramitacao` possuía apenas 988 valores distintos e faltava em 174 linhas; o par `uriProposicao` + `sequencia` se repetia, inclusive com órgãos/despachos diferentes e com cópias exatas. Por isso nenhuma dessas combinações é tratada como chave única, e nenhuma linha é rejeitada ou deduplicada por repetição. A proveniência do recurso e do snapshot Bronze identifica o conjunto processado. A CLI `todeolho refinar silver camara-tramitacoes` publica os anos individualmente e integra `--todas` e a reconciliação Silver.

### Candidaturas do TSE

O contrato `tse.candidatura.registro.v1`, executado por [`tse_candidaturas.sql`](../../crates/refino/sql/silver/tse_candidaturas.sql), publica uma linha por registro do consolidado nacional `BRASIL` em cada recurso eleitoral anual. As partes por UF do mesmo ZIP não são somadas ao consolidado. `ano_eleicao` é `UInt16`; `codigo_eleicao`, `uf`, `unidade_eleitoral`, `codigo_cargo`, `numero_candidato` e `sequencia_candidato` permanecem texto para preservar códigos e zeros à esquerda. A chave candidata é a combinação completa desses campos com o ano do recurso; ela é validada entre lotes e duplicatas interrompem a publicação. A análise Bronze mediu unicidade dessa chave nos 17 anos eleitorais locais, de 1994 a 2026; a chave curta eleição + sequência colide em eleições antigas.

`data_eleicao` e `data_nascimento` são `Date32`; `idade_data_posse` é `UInt8`. Datas não interpretáveis ou fora da faixa plausível e sentinelas de idade são tipadas como nulas, sem remover a linha. `registro_original` conserva todos os campos e nomes publicados naquele ano, inclusive os aliases históricos (por exemplo `NM_EMAIL`/`DS_EMAIL`), sem projetá-los como colunas canônicas antes de uma regra de domínio. A regra `tse-candidatura-registro-v2` reprocessa as saídas Arrow anteriores; a consulta SQL valida a chave em todo o recurso antes de publicar. Use `todeolho refinar silver tse-candidaturas` para processar os recursos disponíveis; a fonte também participa de `silver --todas`.

Smoke local nos 17 consolidados Bronze (1994–2026): 3.703.307 linhas de entrada e saída, sem linhas removidas, 2.428 datas da eleição, 121.623 datas de nascimento e 1.464.963 idades tipadas como nulas (inclui campos ausentes e sentinelas), e zero data da eleição fora da faixa tipada. A chave completa foi validada durante o processamento de cada lote. A segunda reconciliação não republicou nenhum ano, e o modo periódico não registrou execução no painel quando todos já estavam em dia. Com concorrência 2, a execução levou 7min41s; o recurso mais lento foi 2012, em 143,3s. O fluxo limita cada lote a 65.536 registros; esta execução não mediu o pico de RSS, que permanece pendente como evidência operacional da issue #189.

Smoke SQL em bucket isolado (2026-11-07): os 17 recursos publicaram 3.703.307 linhas. A contagem e o multiconjunto SHA-256 de `registro_original` coincidiram com a saída Arrow anterior; os demais anos já estavam na versão SQL e foram reaproveitados. O SQL validou a chave completa em cada recurso e selecionou somente a tabela `BRASIL` pela URL de origem, sem somar as partes por UF. A validação de duplicatas agrupa somente as colunas da chave e reduz o resultado a um escalar, em vez de calcular uma janela sobre linhas completas. Com concorrência 1, o smoke integral levou cerca de 11min40s e o RSS máximo observado por amostragem de 50 ms foi 905.420 KiB (~884 MiB), abaixo do limite de 2 GiB do serviço `refino`.

### Bens declarados do TSE

O contrato `tse.candidatura.bem.v1`, executado por [`tse_bens.sql`](../../crates/refino/sql/silver/tse_bens.sql), publica uma linha por bem no consolidado nacional `BRASIL` de cada eleição, sem somar as partes por UF. A chave composta é (`ano_eleicao`, `codigo_eleicao`, `uf`, `unidade_eleitoral`, `sequencia_candidato`, `ordem_bem`), validada entre lotes. A ordem é lida de `NR_ORDEM_CANDIDATO` nos layouts de 2006–2012 e 2016 e de `NR_ORDEM_BEM_CANDIDATO` em 2014 e 2018–2026; não se infere o alias por uma regra monotônica. A chave eleitoral do bem também permite vinculá-lo à candidatura de mesmo ano, eleição, UF, unidade eleitoral e sequência do candidato.

`ano_eleicao` é `UInt16`, `ordem_bem` é `UInt32`, as datas são `Date32` e `valor_bem` é `Decimal128(18,2)`. Valores são convertidos por aritmética decimal exata a partir da vírgula brasileira. Datas não interpretáveis ou fora da faixa plausível viram nulas sem remover a linha, e `registro_original` preserva todos os campos e aliases de origem; isso é necessário, por exemplo, para valores de última atualização antigos que não são datas válidas. Smoke local nos 11 consolidados (2006–2026): 5.101.128 linhas de entrada e saída, zero valores ou ordens nulos/inválidos, 8.967 datas de eleição nulas e 4.041.301 atualizações nulas; nenhuma linha foi removida. Um join Silver por ano encontrou exatamente uma candidatura para cada bem (zero bens sem correspondência). A segunda passada reconheceu os 11 recursos como atualizados e não os republicou; a primeira execução levou 4min52s com concorrência 2, sem medição de pico de RSS (evidência operacional ainda pendente na issue #189). Use `todeolho refinar silver tse-bens`; a fonte também integra `silver --todas` e sua reconciliação periódica.

Smoke SQL em bucket isolado (2026-11-07): os 11 recursos publicaram 5.101.128 linhas. A contagem e o multiconjunto SHA-256 de `registro_original` coincidiram com a saída Arrow anterior; a chave foi validada pelo contrato SQL e cada ano foi publicado sem alterar a Silver vigente. A validação usa a mesma agregação escalar por chave. Com concorrência 1, o smoke integral levou cerca de 7min13s e o RSS máximo observado por amostragem de 50 ms foi 527.448 KiB (~515 MiB), abaixo do limite de 2 GiB do serviço `refino`.

### Votação por seção do TSE

O contrato `tse.votacao.secao.v1`, executado por [`tse_votacao_secao.sql`](../../crates/refino/sql/silver/tse_votacao_secao.sql) (regra v2), publica uma linha por (`ano_eleicao`, `codigo_eleicao`, `uf`, `turno`, `codigo_municipio`, `zona`, `secao`, `codigo_cargo`, `numero_votavel`) a partir de cada recurso `votacao_secao/ano=AAAA/uf=UF`. Ano e turno são inteiros tipados, votos são `UInt32`, data de eleição é `Date32`; os códigos permanecem texto e todos os campos de origem são preservados em `registro_original`. A unicidade é verificada por uma agregação escalar de chaves com prefixo de comprimento, evitando a janela global sobre as linhas completas; duplicatas interrompem a publicação. A tarefa usa pool de memória DataFusion de 256 MiB e limite de spill de 8 GiB por recurso.

Os recursos `BR` e estaduais são complementares: o primeiro contém votos presidenciais e os segundos cargos estaduais. A tarefa mantém ambos, validando que cada linha estadual pertence à UF do recurso. Os recursos 1998/BR e 2010/VT e 2010/ZZ são excluídos porque a Bronze atual não recupera seus cabeçalhos e a chave padrão não está disponível. O TSE informa que os resultados de 1998 são incompletos ([catálogo oficial](https://dadosabertos.tse.jus.br/dataset/resultados-1998)); esses três recursos precisam de trabalho separado na decodificação Bronze antes de entrarem no contrato. O restante do histórico precisa ser monitorado contra mudanças nos layouts publicados.

Em smokes release no cgroup de 2 GiB (2026-11-07), a chave escalar compacta concluiu para todas as 26 UFs com snapshot Bronze disponível em 2024; DF não gerou tarefa. Os picos cgroup por UF estão detalhados em [`benchmark-datafusion.md`](benchmark-datafusion.md). SP foi o caso mais pesado: 781.508.516 bytes publicados, 134 s e pico de 2.099.367.936 bytes (1,95 GiB); uma repetição para validar a UI concluiu em 136,5 s, com pico de 2.133.766.144 bytes (1,99 GiB), sem OOM. Essa execução aparece no histórico normal da UI como `20261107T212424Z-23c97421`; somente o histórico foi escrito no bucket `execucoes`, enquanto o Parquet permaneceu em bucket Silver descartável. Nenhum ponteiro Silver vigente foi alterado. O histórico completo de votação ocupa cerca de 16,4 GB no Bronze e os demais anos ainda aguardam smoke com o plano atualizado.

Execute explicitamente com `todeolho refinar silver tse-votacao-secao`. A coleção local consultada ocupa cerca de 16,4 GB, então não participa de `silver --todas` ou da sua reconciliação periódica. Para smoke parcial, use `--filtro ano=2024 --filtro uf=AC,RO`; ambos os filtros são opcionais e repetíveis, e os valores aceitam listas separadas por vírgula. Votos por seção anteriores a 1998 não são publicados pelo TSE. A cobertura da chave foi validada no Bronze em AC e RO de 1998–2026. No smoke Silver 2024, AC e RO publicaram 156.259 e 349.871 linhas respectivamente, contagens iguais às partes Bronze e sem duplicatas de chave composta. Três reconciliações seguintes reconheceram ambos recursos como em dia.

### Despesas do Senado (CEAPS)

O contrato `senado.despesa.ceaps.v1`, em [`senado_despesas.sql`](../../crates/refino/sql/silver/senado_despesas.sql), tipa `ANO` do recurso como `UInt16`, `MES` como `UInt8`, `DATA` como `Date32` e `VALOR_REEMBOLSADO` como `Decimal128(18,2)`. CNPJ/CPF, fornecedor, documento e `COD_DOCUMENTO` continuam texto para preservar zeros à esquerda. A conversão decimal SQL usa ponto apenas como formato intermediário após validar a vírgula brasileira, sem `f64`; aceita frações de zero a duas casas, `,95`, negativos e quebras CR/LF embutidas no valor histórico de 2013. No Bronze local havia 398.172 linhas de 2008 a 2026; `COD_DOCUMENTO` foi único em 398.170 linhas e estava ausente em duas. O código é validado quando preenchido; registros sem código não são removidos nem deduplicados. Dois registros de 2015 têm aspas malformadas no campo `DOCUMENTO` da origem CSV, o que desloca colunas no Bronze: preservamos essas linhas e os campos Bronze em `registro_original`, mas deixamos data e valor tipados nulos para não interpretar texto deslocado como despesa. Para as demais linhas, a Silver deixa `DATA` tipada nula quando o ano é anterior a 1900 ou posterior ao ano de competência + 1, preservando o texto de origem; essa faixa cobre as 75 datas parseadas fora dos limites encontradas na amostra Bronze local. Valores negativos permanecem com o sinal recebido. A regra `senado-despesa-ceaps-v4` reprocessa os snapshots publicados pelo transformador Arrow e mantém a política atual. Os testes comparam a saída SQL com a saída Arrow para o fixture CEAPS. Smoke local em 2026-11-07 publicou os 19 anos disponíveis no Bronze (2008–2026), e a segunda reconciliação não alterou nenhum ano. A CLI `todeolho refinar silver senado-despesas` publica cada ano e participa de `--todas` e da reconciliação Silver.

- O decodificador de agregados IBGE já produz as dez colunas `variavel_id`, `variavel`, `unidade`, `classificacoes`, `nivel_id`, `nivel_nome`, `localidade_id`, `localidade_nome`, `periodo` e `valor`. O fixture mostra `nivel_id=N6` e código municipal `1100015`, ambos a preservar como texto.
- O fixture do BCB traz `data=01/06/1989` e `valor=19.68`; converter data e decimal exige regra explícita. O CSV de despesas do Senado exemplifica `52,38`, então o parser numérico não pode ser global para todas as fontes.
- A ingestão de votações da Câmara declara sete conjuntos anuais. O bronze CSV recebe o nome de tabela `votacoes`, mas o conjunto está no caminho do recurso; um contrato silver único para essa entrada misturaria grãos diferentes.
- O detalhe de processo do Senado vira uma linha por objeto, com campos aninhados em JSON compacto. O fixture confirma `id` e `conteudo` aninhado, mas não prova a unicidade entre recursos/anos.
- A ingestão do Ipeadata declara séries municipais e `ivs` regional/estadual, mas a amostra bronze mostra também linhas nacionais e estaduais nas séries ADH. O nível geográfico precisa vir de `NIVNOME` por observação, sem presumir município pelo código da série.

### Amostras locais do bronze (2026-11-05)

O armazenamento local contém Parquet para **todas as 26 entradas**. Inspecionei uma parte de cada entrada com DuckDB; isso confirma presença e nomes de colunas, mas não valida chaves, nulos ou variação entre anos/tabelas internas. Exemplos relevantes:

| Entrada | Colunas observadas na amostra | Conclusão para o contrato |
| --- | --- | --- |
| `bcb` | `data`, `valor` | `data` e `valor` chegam como texto; `selic-mensal` tem 483 linhas e 483 datas distintas na amostra consultada. |
| `ibge-pib-municipios` | `variavel_id`, `classificacoes`, `nivel_id`, `localidade_id`, `periodo`, `valor` | A amostra consultada tem 122.540 linhas, todas `nivel_id=N6`; a chave ainda precisa incluir classificações. |
| `ipeadata` | `NIVNOME`, `SERCODIGO`, `TERCODIGO`, `VALDATA`, `VALVALOR` | ADH_GINI traz Brasil (3), estados (81) e municípios (16.692); AVS_IVS traz Brasil (16), estados (432) e regiões (80). |
| `camara-votacoes` | `idVotacao`, `uriVotacao`, `siglaBancada`, `orientacao` | A parte inspecionada corresponde a orientação; os outros seis conjuntos exigem contratos próprios. |
| `tse-prestacao-contas` | `Cód. Eleição`, `Data e hora`, `Valor despesa` | A parte inspecionada usa cabeçalhos antigos com espaços/acentos; não serve de schema para todas as eleições. |
| `tse-votacao-secao` | `CD_MUNICIPIO`, `NR_ZONA`, `NR_SECAO`, `NR_VOTAVEL`, `QT_VOTOS` | A parte inspecionada permite formular chave candidata por eleição/turno/seção/cargo/votável; validar duplicatas e recortes por UF. |

As consultas acima são somente leitura. Não foram inspecionados registros individuais de candidaturas ou prestação de contas.

## Validação de chaves no bronze local

As contagens abaixo foram obtidas com DuckDB sobre os Parquets locais, sem retornar registros individuais. **Não são prova de chave universal**: os globais podem incluir versões históricas, e os recortes TSE cobrem somente as partes indicadas. O contrato silver deve usar apenas a versão vigente de cada recurso e repetir a validação em anos/layouts diferentes.

| Tabela e recorte | Linhas | Chaves distintas | Resultado |
| --- | ---: | ---: | --- |
| BCB `selic-mensal`, `data` | 483 | 483 | Sem nulos; todas as 11 séries locais foram conferidas abaixo. |
| Ipeadata, (`SERCODIGO`, `TERCODIGO`, `VALDATA`) | 67.632 | 67.632 | Sem nulos no conjunto local consultado. |
| IBGE estados, `id` | 27 | 27 | Sem nulos no conjunto local consultado. |
| IBGE regiões, `id` | 5 | 5 | Sem nulos no conjunto local consultado. |
| IBGE municípios, `id` | 5.571 | 5.571 | Sem nulos no conjunto local consultado. |
| IBGE IPCA, chave com variável, classificações, nível, localidade e período | 562 | 562 | Sem nulos nos campos variável, localidade e período. |
| IBGE INPC, mesma chave | 571 | 571 | Mesma condição. |
| IBGE PIB municipal, mesma chave | 122.540 | 122.540 | Mesma condição; `nivel_id=N6` na amostra local. |
| IBGE PIB trimestral, mesma chave | 122 | 122 | Mesma condição. |
| IBGE PNAD desocupação, mesma chave | 1.566 | 1.566 | Mesma condição. |
| IBGE população municipal, mesma chave | 122.562 | 122.562 | Mesma condição. |
| Câmara deputados, `id` | 513 | 513 | Sem nulos no conjunto local consultado; vigência/histórico ainda exigem contrato. |
| Câmara partidos, `id` | 21 | 21 | Sem nulos no conjunto local consultado. |
| Câmara legislaturas, `id` | 57 | 57 | Sem nulos no conjunto local consultado. |
| Senado parlamentares, (`IdentificacaoParlamentar.CodigoParlamentar`, `legislatura`) | 2.035 | 2.035 | Código extraído do JSON compacto; sem nulos no conjunto local consultado. |
| Senado votações, `codigoSessaoVotacao` | 3.652 | 3.652 | Sem nulos no conjunto local consultado; conferir semântica em outros anos. |
| Senado processos, `id`, ano 2024 | 4.947 | 4.947 | Único no recorte; glob completo contém Parquet antigo de zero colunas. |
| Senado processos-detalhes, `id` | 163.156 | 163.156 | Sem nulos no conjunto local consultado. |
| TSE votação por seção 2014/AC, (`CD_ELEICAO`, `NR_TURNO`, `CD_MUNICIPIO`, `NR_ZONA`, `NR_SECAO`, `CD_CARGO`, `NR_VOTAVEL`) | 224.661 | 224.661 | Única no recorte; validação histórica de AC e RO abaixo. |
| TSE candidaturas 2014, uma parte, (`CD_ELEICAO`, `SQ_CANDIDATO`) | 879 | 879 | Única na parte; validação dos consolidados por ano abaixo. |
| TSE bens 2014, uma parte, (`CD_ELEICAO`, `SQ_CANDIDATO`, `NR_ORDEM_BEM_CANDIDATO`) | 3.588 | 3.588 | Única na parte; validação dos consolidados por ano abaixo. |

### Séries BCB no bronze local (2026-11-05)

Há uma versão Parquet por recurso nas 11 séries disponíveis. Em cada série, `data` é única e não vazia. Todas as 5.125 datas são interpretáveis por `%d/%m/%Y`, e todos os `valor` são conversíveis para `DECIMAL(18,6)` no DuckDB. A chave candidata da silver é (`recurso`, `data`); o recurso distingue séries com a mesma data. Esta validação cobre somente as versões locais observadas, não futuras correções ou novas séries.

| Série | Linhas | Datas distintas |
| --- | ---: | ---: |
| `cambio-dolar-mensal` | 885 | 885 |
| `cdi-mensal` | 483 | 483 |
| `credito-total-pib` | 375 | 375 |
| `divida-liquida-setor-publico-pib` | 298 | 298 |
| `ibc-br` | 284 | 284 |
| `igpm-mensal` | 449 | 449 |
| `inadimplencia-credito-total` | 187 | 187 |
| `inpc-mensal` | 570 | 570 |
| `ipca-12-meses` | 550 | 550 |
| `ipca-mensal` | 561 | 561 |
| `selic-mensal` | 483 | 483 |

### Candidaturas e bens do TSE no bronze local (2026-11-05)

As contagens abaixo usam **somente** a parte `BRASIL` indicada pelo manifesto vigente de cada ano. Os ZIPs também contêm partes por UF; somá-las ao consolidado duplica candidaturas. Na amostra de 2000, as partes por UF têm 457.171 linhas e 399.455 chaves contextuais, enquanto `BRASIL` tem 399.455 linhas e chaves. Os conjuntos de chaves das duas representações são iguais, mas cinco partes por UF repetem internamente 57.716 chaves com linhas diferentes. A Silver precisa escolher uma representação e definir a política para divergências de atributos antes de publicar candidatos.

Para candidaturas, a chave curta (`CD_ELEICAO`, `SQ_CANDIDATO`) colide em 2000, 2002, 2004, 2006 e 2008. A chave candidata completa é (`ANO_ELEICAO`, `CD_ELEICAO`, `SG_UF`, `SG_UE`, `CD_CARGO`, `NR_CANDIDATO`, `SQ_CANDIDATO`). Nos 17 consolidados, ela é única por ano e seus campos não são SQL `NULL`. `ANO_ELEICAO` delimita anos distintos; a tabela mostra a contagem da chave sem ele, por já estar agrupada por ano.

| Ano | Linhas `BRASIL` | Chave curta distinta | Chave completa distinta |
| --- | ---: | ---: | ---: |
| 1994 | 12.035 | 12.035 | 12.035 |
| 1996 | 321.790 | 321.790 | 321.790 |
| 1998 | 15.092 | 15.092 | 15.092 |
| 2000 | 399.455 | 3.243 | 399.455 |
| 2002 | 18.109 | 5.169 | 18.109 |
| 2004 | 402.157 | 1.874 | 402.157 |
| 2006 | 19.303 | 3.205 | 19.303 |
| 2008 | 382.079 | 69.286 | 382.079 |
| 2010 | 22.577 | 22.577 | 22.577 |
| 2012 | 483.741 | 483.741 | 483.741 |
| 2014 | 26.263 | 26.263 | 26.263 |
| 2016 | 498.391 | 498.391 | 498.391 |
| 2018 | 29.287 | 29.287 | 29.287 |
| 2020 | 558.804 | 558.804 | 558.804 |
| 2022 | 29.322 | 29.322 | 29.322 |
| 2024 | 463.885 | 463.885 | 463.885 |
| 2026 | 21.017 | 21.017 | 21.017 |

Nos bens, o campo de ordem é `NR_ORDEM_CANDIDATO` em 2006–2012 e 2016, e `NR_ORDEM_BEM_CANDIDATO` em 2014 e 2018–2026. A mudança **não é monotônica por ano**. A chave candidata (`ANO_ELEICAO`, `CD_ELEICAO`, `SG_UF`, `SG_UE`, `SQ_CANDIDATO`, ordem) é única por ano nos 11 consolidados; seus campos não são SQL `NULL`. A chave curta (`CD_ELEICAO`, `SQ_CANDIDATO`, ordem) colide em 2006 e 2008. A associação (`ANO_ELEICAO`, `CD_ELEICAO`, `SG_UF`, `SG_UE`, `SQ_CANDIDATO`) encontrou uma candidatura única para cada linha de bem nos 11 anos locais.

| Ano | Bens `BRASIL` | Chave curta distinta | Chave completa distinta | Sem candidatura |
| --- | ---: | ---: | ---: | ---: |
| 2006 | 82.361 | 33.317 | 82.361 | 0 |
| 2008 | 791.319 | 345.796 | 791.319 | 0 |
| 2010 | 81.226 | 81.226 | 81.226 | 0 |
| 2012 | 895.663 | 895.663 | 895.663 | 0 |
| 2014 | 83.053 | 83.053 | 83.053 | 0 |
| 2016 | 977.510 | 977.510 | 977.510 | 0 |
| 2018 | 93.527 | 93.527 | 93.527 | 0 |
| 2020 | 1.015.279 | 1.015.279 | 1.015.279 | 0 |
| 2022 | 92.559 | 92.559 | 92.559 | 0 |
| 2024 | 911.185 | 911.185 | 911.185 | 0 |
| 2026 | 77.446 | 77.446 | 77.446 | 0 |

Esses números verificam as versões locais vigentes, não mudanças futuras da origem. Antes de materializar, validar também sentinelas de texto, correções entre versões e a cobertura do consolidado em todos os anos. As partes por UF de 2000 mostram que unicidade de chave não resolve sozinha divergências de atributos.

### Votação por seção do TSE no bronze local (2026-11-05)

Cada recurso `ano` × `uf` de AC e RO tem uma única versão Parquet local. Nas 30 partes desses estados, de 1998 a 2026, a chave candidata (`ANO_ELEICAO`, `CD_ELEICAO`, `SG_UF`, `NR_TURNO`, `CD_MUNICIPIO`, `NR_ZONA`, `NR_SECAO`, `CD_CARGO`, `NR_VOTAVEL`) é única por ano e não tem campos SQL `NULL` nem vazios; `SG_UF` concorda com o recurso. A tabela mostra a contagem da chave sem `ANO_ELEICAO`, pois já está agrupada por ano. Acrescentar `SG_UF` explicita o escopo geográfico, mesmo que ela não tenha sido necessária para distinguir as linhas de AC e RO.

| Ano | Linhas AC + RO | Chaves distintas |
| --- | ---: | ---: |
| 1998 | 350.606 | 350.606 |
| 2000 | 253.934 | 253.934 |
| 2002 | 475.169 | 475.169 |
| 2004 | 309.136 | 309.136 |
| 2006 | 485.207 | 485.207 |
| 2008 | 344.724 | 344.724 |
| 2010 | 544.097 | 544.097 |
| 2012 | 434.002 | 434.002 |
| 2014 | 633.544 | 633.544 |
| 2016 | 463.463 | 463.463 |
| 2018 | 797.825 | 797.825 |
| 2020 | 484.535 | 484.535 |
| 2022 | 788.644 | 788.644 |
| 2024 | 506.130 | 506.130 |
| 2026 | 754.814 | 754.814 |

Como recorte de uma UF grande, SP/2024 tem 9.611.657 linhas e o mesmo número de chaves distintas, sem campos da chave vazios ou UF divergente. A regra ainda precisa ser validada nos outros recursos, inclusive `uf=BR` nos anos em que existe, e em versões futuras. A Silver deve ler a versão apontada por `_ultima_versao.json` de cada recurso, não um glob que reúna execuções antigas.

### Prestação de contas do TSE em 2024

O manifesto vigente de `tse/prestacao-contas/ano=2024` contém quatro conjuntos diferentes, cada um com 26 partes por UF e um consolidado `BRASIL`: `despesas_contratadas_candidatos`, `receitas_candidatos`, `despesas_pagas_candidatos` e `receitas_candidatos_doador_originario`. Cada conjunto precisa de contrato Silver próprio. As contagens abaixo usam **somente** a parte `BRASIL` de cada um; nenhum identificador pessoal foi exibido.

| Conjunto | Chave sequencial testada, além de `CD_ELEICAO` e `SQ_PRESTADOR_CONTAS` | Linhas | Chaves distintas | Hashes de linha distintos |
| --- | --- | ---: | ---: | ---: |
| Despesas contratadas | `SQ_DESPESA` | 4.452.310 | 3.125.686 | 4.380.965 |
| Receitas | `SQ_RECEITA` | 2.041.059 | 1.898.067 | 2.038.794 |
| Despesas pagas | (`SQ_DESPESA`, `SQ_PARCELAMENTO_DESPESA`) | 3.719.729 | 3.719.650 | 3.719.650 |
| Doador originário | `SQ_RECEITA` | 534.344 | 452.567 | 533.011 |

Os campos das chaves testadas não são SQL `NULL`, mas o [leiaute oficial de 2024 e a análise das variantes](pesquisa-tse-2024.md) mostram sentinelas `-1`: 159.100 sequências de despesas contratadas, 66.599 de receitas e 433.385 pares de sequência/documento de doador originário. As colisões persistem após excluí-las. Nenhuma das quatro combinações é única. Acrescentar `TP_PRESTACAO_CONTAS` ou `DT_PRESTACAO_CONTAS` não resolve as três primeiras; em doador originário, é o **documento**, não a data, que eleva a cardinalidade para 529.130. Hashes de linha mostram variantes de atributos em despesas contratadas, receitas e doador originário, mas não são chave de negócio nem regra de descarte.

Nas 56 chaves repetidas de despesas pagas, a comparação exata encontrou 135 linhas e 56 linhas distintas: as 79 excedentes são cópias idênticas no consolidado local. A tarefa Silver de `despesas_pagas_candidatos` 2024 executa [`tse_despesas_pagas.sql`](../../crates/refino/sql/silver/tse_despesas_pagas.sql) (regra v2) e usa (`AA_ELEICAO`, `CD_ELEICAO`, `SQ_PRESTADOR_CONTAS`, `SQ_DESPESA`, `SQ_PARCELAMENTO_DESPESA`), remove somente registros integrais iguais e falha se uma chave tiver conteúdo diferente. `SQ_PRESTADOR_CONTAS` permanece texto porque seus valores não cabem em `UInt32`; despesas pagas, data e valor são tipados. Smoke na parte `BRASIL`: 3.719.729 linhas de entrada, 3.719.650 de saída, 79 cópias exatas removidas, zero nulos em data/valor e zero chaves repetidas. A execução no perfil de desenvolvimento levou 525,5 s; o maior RSS amostrado foi cerca de 1,07 GB. A reconciliação seguinte reconheceu o recurso em dia. O contrato SQL agrupa pela chave e compara SHA-256 do registro integral, removendo as cópias e interrompendo conflitos; não emite a métrica por linha que a implementação Arrow registrava. Smoke SQL em bucket isolado (2026-11-07) publicou as mesmas 3.719.650 linhas; o multiconjunto SHA-256 de `registro_original` coincidiu com o Arrow, e data e valor tiveram zero nulos nas duas saídas. O snapshot carregou somente a parte `BRASIL`. Um teste com pool de 1 GiB e spill de 8 GiB usou cerca de 3,7 GB de spill, mas atingiu RSS de 2.213.732 KiB (~2,11 GiB), acima do limite de 2 GiB do container. Com pool de 512 MiB e spill de 8 GiB, o smoke final isolado levou 501,8 s, publicou as mesmas 3.719.650 linhas e teve RSS local máximo de 1.723.348 KiB (~1,64 GiB). No container com limite de 2 GiB, a tarefa concluiu sem OOM, mas o pico cgroup atingiu o teto exato. O pool foi reduzido para 256 MiB; novo smoke release em container limitado a 2 GiB concluiu em 70,7 s, sem OOM, e publicou novamente 154.370.020 bytes. O pico cgroup ainda atingiu o teto exato, sem redução observável em relação ao pool de 512 MiB; portanto, a tarefa conclui sob esse limite, mas o cgroup fica saturado durante o processamento. Nenhuma dessas execuções alterou o ponteiro Silver vigente. O pico de spill não foi medido na passada final.

Receitas de candidatos de 2024 usam o contrato [`tse_receitas_candidatos.sql`](../../crates/refino/sql/silver/tse_receitas_candidatos.sql), com saída em `prestacao-contas/ano=2024/receitas-candidatos`. O contrato preserva cada linha do consolidado `BRASIL`, inclusive repetições de `SQ_RECEITA` com atributos diferentes; não trata sequência como chave única nem elimina cópias. `SQ_PRESTADOR_CONTAS` fica como texto, `SQ_RECEITA` vira `UInt32` anulável (`-1`/`-3` são sentinelas), e data e valor viram `Date32` anulável e `Decimal128(18,2)`. Datas ausentes ou sentinelas permanecem nulas, com o texto original preservado em `registro_original`. A identidade técnica da linha é a linhagem da versão Bronze (`manifesto`, `parte`, `ordinal`); o registro original mantém os demais campos do TSE. A fixture executa o contrato pela publicação Silver e cobre sequências/data sentinela, variantes de descrição/valor e decimais brasileiros. Smoke de volume integral no bucket local (2026-11-10) publicou 2.041.059 linhas em uma parte de 223.309.042 bytes; a reconciliação levou 876 s no binário de desenvolvimento. Na mesma execução, despesas pagas publicaram 3.719.650 linhas em uma parte de 167.886.944 bytes, em 884 s. As contagens foram conferidas pelas views Silver do console DuckDB; esta passada não mediu recursos de forma controlada.

Despesas contratadas de 2024 usam [`tse_despesas_contratadas.sql`](../../crates/refino/sql/silver/tse_despesas_contratadas.sql) e publicam em `prestacao-contas/ano=2024/despesas-contratadas`. O contrato lê somente a parte `BRASIL`, preserva todas as linhas mesmo quando `SQ_DESPESA` se repete, mantém `SQ_PRESTADOR_CONTAS` como texto e publica sequência como `UInt32` anulável, data como `Date32` anulável e valor como `Decimal128(18,2)`. Descrições vazias e datas ausentes/sentinelas viram nulas; os campos originais permanecem em `registro_original`. A linhagem (`manifesto`, `parte`, `ordinal`) distingue cada ocorrência dentro da versão Bronze. A fixture cobre colisões de sequência, sentinelas, decimal brasileiro, preservação do original e exclusão da parte estadual. Smoke integral no bucket local (2026-11-10) publicou 4.452.310 linhas em uma parte de 544.820.172 bytes, em 113,9 s no binário release. A contagem pelo console confirmou 159.100 sequências sentinela e datas nulas, duas descrições vazias e zero valores nulos; não foi uma medição controlada de memória.

Receitas de doador originário de 2024 usam [`tse_receitas_doador_originario.sql`](../../crates/refino/sql/silver/tse_receitas_doador_originario.sql) e publicam em `prestacao-contas/ano=2024/receitas-doador-originario`. A tarefa seleciona somente `BRASIL` e mantém todas as ocorrências, inclusive colisões em sequência e documento. Documento permanece texto para preservar zeros à esquerda; sequência, data e valor são tipados, sentinelas de documento, sequência, data e campos descritivos viram nulas, e o registro original permanece em `registro_original`. Nome e documento são dados pessoais publicados pelo TSE e seguem disponíveis apenas nas colunas da relação e no campo original. A linhagem identifica cada linha dentro da versão Bronze. A fixture usa valores sintéticos e cobre variantes, sentinelas e exclusão da parte estadual. Smoke integral no bucket local publicou 534.344 linhas em uma parte de 16.412.400 bytes, em 4,9 s no binário release. A conferência agregada confirmou 433.385 linhas com documento, sequência e data nulos e zero valores nulos; nenhum dado pessoal foi consultado durante a verificação.

Receitas de doador originário de 2022 usam [`tse_receitas_doador_originario_2022.sql`](../../crates/refino/sql/silver/tse_receitas_doador_originario_2022.sql), publicam em `prestacao-contas/ano=2022/receitas-doador-originario` e leem somente o consolidado `_BRASIL`. O inventário de 2022 confirma que `_BRASIL` equivale às UFs + `_BR`; a combinação de sequência/documento continua colidindo e 2.289 ocorrências são repetidas integralmente, portanto nenhuma linha é removida. O contrato mantém documento como texto, tipa sequência, data e valor, preserva o original e identifica ocorrências pela linhagem Bronze. A fixture cobre variantes, sentinelas e exclusão da parte estadual. Smoke integral no bucket local publicou 199.241 linhas em uma parte de 13.691.354 bytes, em 2,4 s no binário release. A conferência agregada confirmou 25.011 nulos em documento, sequência e data, nenhum valor nulo e ano 2022 em todas as linhas; nenhum dado pessoal foi consultado durante a verificação.

Despesas de candidatos de 2016 usam [`tse_despesas_candidatos_2016.sql`](../../crates/refino/sql/silver/tse_despesas_candidatos_2016.sql) e publicam em `prestacao-contas/ano=2016/despesas-candidatos`. O layout de 2016 antecede a separação entre contratadas e pagas introduzida em 2018: há um único consolidado `despesas_candidatos_prestacao_contas_final_2016_BRASIL` com cabeçalhos acentuados e espaçados (ex.: `Cód. Eleição`, `Data da despesa`, `Valor despesa`, `Descriçao da despesa`). Datas vêm coladas ao horário (`dd/MM/yyyyHH:mm:ss`); valores são inteiros ou decimais com vírgula (ex.: `1029`, `100,01`, `190,7`). O contrato seleciona somente `_BRASIL`, reconhece o formato completo de data e horário e converte a data em `Date32`, normaliza vírgula em ponto para `Decimal128(18,2)` e insere o ano literal 2016 (a fonte não tem coluna de ano). Código de eleição, sequencial de candidato e CPF de candidato são obrigatórios; data e valor são validados por assert. O registro original e a linhagem Bronze (manifesto, parte, ordinal) acompanham cada linha. Smoke integral no armazenamento local (2026-11-12) processou 5.588.616 linhas, sem datas ou valores nulos após a conversão.

Receitas de candidatos de 2016 usam [`tse_receitas_candidatos_2016.sql`](../../crates/refino/sql/silver/tse_receitas_candidatos_2016.sql) e publicam em `prestacao-contas/ano=2016/receitas-candidatos`. O layout segue as mesmas peculiaridades de 2016: cabeçalhos acentuados (`Cód. Eleição`, `Data da receita`, `Valor receita`, `Descricao da receita`), datas coladas ao horário (`dd/MM/yyyyHH:mm:ss`), valores inteiros ou decimais com vírgula e sem coluna de ano. O contrato seleciona somente `_BRASIL`, reconhece o formato completo de data e horário e converte a data em `Date32`, normaliza vírgula em ponto para `Decimal128(18,2)` e insere o ano literal 2016. Código de eleição, sequencial de candidato e CPF de candidato são obrigatórios; data e valor são validados por assert. O registro original e a linhagem Bronze (manifesto, parte, ordinal) acompanham cada linha. Smoke integral no armazenamento local (2026-11-12) processou 3.004.707 linhas, sem datas ou valores nulos após a conversão.

Despesas de partidos de 2016 usam [`tse_despesas_partidos_2016.sql`](../../crates/refino/sql/silver/tse_despesas_partidos_2016.sql) e publicam em `prestacao-contas/ano=2016/despesas-partidos`. O layout difere do de candidatos: a data usa `DD-MON-YY` com mês abreviado em inglês, e o prestador é o diretório partidário: as colunas-chave são `CNPJ Prestador Conta` e `Sequencial do Prestador de conta` em vez de CPF e sequencial de candidato. A coluna de descrição é `Descrição da despesa` (com til, diferente de despesas de candidatos que traz `Descriçao` — typo no original). O contrato seleciona somente `_BRASIL`, converte a data com formato explícito, normaliza o valor decimal e insere o ano literal 2016. Código de eleição, sequencial do prestador e CNPJ do prestador são obrigatórios; data e valor são validados por assert. O registro original e a linhagem Bronze acompanham cada linha. A fixture valida cópias exatas, data `DD-MON-YY`, valor com vírgula inicial ou negativo e exclusão da parte estadual. No Bronze nacional medido, as 282.727 datas usam essa máscara e foram convertidas em consulta agregada. Smoke integral no armazenamento local (2026-11-12) publicou 282.727 linhas em uma parte de 17.240.400 bytes, em 24,2 s no binário de desenvolvimento. DuckDB confirmou 282.727 ordinais distintos, zero datas e valores nulos e ano 2016 em todas as linhas.

Receitas de partidos de 2016 usam [`tse_receitas_partidos_2016.sql`](../../crates/refino/sql/silver/tse_receitas_partidos_2016.sql) e publicam em `prestacao-contas/ano=2016/receitas-partidos`. O layout usa datas `DD-MON-YY` com mês abreviado em inglês, diferente das receitas de candidatos. O prestador é identificado por `CNPJ Prestador Conta` e `Sequencial prestador conta`. A coluna de descrição é `Descrição da receita` (com acentos corretos). O contrato seleciona somente `_BRASIL`, converte a data com formato explícito, normaliza o valor decimal e insere o ano literal 2016. O inventário do Bronze nacional confirmou 140.496 linhas com a máscara `DD-MON-YY`; o smoke integral no armazenamento local (2026-11-12) processou todas elas, sem datas ou valores nulos após a conversão.

Receitas de doador originário de 2018 usam [`tse_receitas_doador_originario_2018.sql`](../../crates/refino/sql/silver/tse_receitas_doador_originario_2018.sql) e publicam em `prestacao-contas/ano=2018/receitas-doador-originario`. O contrato seleciona somente `_BRASIL`, equivalente às UFs mais `_BR` no inventário, e preserva as 151.448 ocorrências, incluindo 1.409 cópias exatas, com registro original e linhagem Bronze. Documento e códigos permanecem texto; sequência e data sentinela tornam-se nulas e valor `Decimal128(18,2)` é obrigatório. A análise agregada do Bronze encontrou 23.813 sequências, datas e documentos ausentes, nenhum valor inválido e valores de R$ 0,00 a R$ 86.007.939,55. A fixture cobre variantes, cópia exata, sentinelas e exclusão da parte estadual. Smoke integral no bucket local (2026-11-11) publicou 151.448 linhas em uma parte de 10.079.855 bytes, em 1,8 s no binário release. DuckDB confirmou 23.813 documentos, sequências e datas nulos, zero valores nulos, ordinais distintos em todas as linhas e ano 2018 em todo o conjunto; nenhum dado pessoal foi consultado durante a verificação.

Receitas de doador originário de 2020 usam [`tse_receitas_doador_originario_2020.sql`](../../crates/refino/sql/silver/tse_receitas_doador_originario_2020.sql) e publicam em `prestacao-contas/ano=2020/receitas-doador-originario`. O contrato lê somente o consolidado `_BRASIL`, preserva as 662.577 ocorrências e as 1.084 cópias exatas medidas no Bronze, com manifesto, parte e ordinal de origem. Documento e códigos permanecem texto; sequência e data sentinela são anuláveis, e valor é `Decimal128(18,2)` obrigatório. A análise agregada do Bronze encontrou 517.474 sequências `-1` e datas vazias, zero valores inválidos e zero datas de calendário inválidas. A fixture inclui uma cópia exata e uma variante sob a mesma sequência, além de sentinelas e uma parte estadual excluída. Smoke integral no bucket local (2026-11-11) publicou 662.577 linhas em uma parte de 21.851.631 bytes, em 6,2 s no binário release. DuckDB confirmou 517.474 sequências e datas nulas, nenhum valor nulo, ordinais distintos em todas as linhas, ano 2020 em todo o conjunto e valores de R$ 0,00 a R$ 750.000,00.

Receitas de candidatos de 2018 usam [`tse_receitas_candidatos_2018.sql`](../../crates/refino/sql/silver/tse_receitas_candidatos_2018.sql) e publicam em `prestacao-contas/ano=2018/receitas-candidatos`. O contrato seleciona somente `_BRASIL`, equivalente às UFs mais `_BR` no inventário, e preserva as 329.683 ocorrências, incluindo 282 cópias exatas, com manifesto, parte e ordinal da origem. Sequência `-1` e data vazia tornam-se nulas; valor `Decimal128(18,2)` é obrigatório. A análise agregada do Bronze encontrou 2.367 sequências e datas ausentes, sem valores inválidos. A fixture inclui variante e cópia exata sob a mesma sequência, além de sentinelas. Smoke integral no bucket local (2026-11-11) publicou 329.683 linhas em uma parte de 35.943.448 bytes, em 10 s no binário release. DuckDB confirmou 2.367 sequências e datas nulas, nenhum valor nulo, ordinais distintos em todas as linhas, ano 2018 em todo o conjunto e valores de R$ 0,00 a R$ 43.369.319,41.

Receitas de candidatos de 2020 usam [`tse_receitas_candidatos_2020.sql`](../../crates/refino/sql/silver/tse_receitas_candidatos_2020.sql) e publicam em `prestacao-contas/ano=2020/receitas-candidatos`. O contrato seleciona somente `receitas_candidatos_2020_BRASIL`, preserva todas as ocorrências (inclusive as 1.432 cópias exatas medidas no snapshot) e registra manifesto, parte e ordinal da origem. Sequências sentinela ou ausentes e datas ausentes tornam-se nulas; valores são validados como `Decimal128(18,2)`, e o registro original permanece disponível. A fixture cobre as sequências `-1`, `-3` e vazia, data ausente e ocorrências com conteúdo distinto. Smoke integral no bucket local (2026-11-11) publicou 2.339.073 linhas em uma parte de 263.308.379 bytes, em 62,7 s no binário release. DuckDB confirmou 93.511 sequências e datas nulas, nenhum valor nulo, ordinais distintos para todas as linhas, ano 2020 em todo o conjunto e valores de R$ 0,00 a R$ 207.166.666,66.

Receitas de candidatos de 2022 usam [`tse_receitas_candidatos_2022.sql`](../../crates/refino/sql/silver/tse_receitas_candidatos_2022.sql), publicam em `prestacao-contas/ano=2022/receitas-candidatos` e selecionam somente `_BRASIL`, cujo conteúdo coincide com UFs + `_BR` no inventário fixado. O contrato preserva as 301 repetições exatas do consolidado, tipa sequência, data e valor, conserva o registro original e a linhagem Bronze. Sentinelas de sequência viram nulas; datas que não correspondem aos formatos aceitos também são nulas. A fixture cobre a preservação de variantes e sentinelas. Smoke integral no bucket local publicou 674.944 linhas em uma parte de 55.929.517 bytes, em 17,5 s no binário release. A verificação agregada confirmou 2.978 nulos em sequência e data, nenhum valor nulo e ano 2022 em todas as linhas; nenhum dado pessoal foi consultado.

Despesas contratadas de 2022 usam [`tse_despesas_contratadas_2022.sql`](../../crates/refino/sql/silver/tse_despesas_contratadas_2022.sql), publicam em `prestacao-contas/ano=2022/despesas-contratadas` e selecionam somente `_BRASIL`. O inventário confirma que o consolidado coincide com UFs + `_BR`, mas a chave sequencial colide; as 43.051 repetições/variantes são preservadas. O contrato tipa sequência, data e valor e guarda registro original e linhagem Bronze. Sentinelas em sequência/data viram nulas; valor fora do formato validado impede publicação. A fixture cobre variantes e sentinelas. Smoke integral no bucket local publicou 2.209.835 linhas em uma parte de 194.573.135 bytes, em 53,7 s no binário release. A conferência agregada confirmou 4.946 sequências e datas nulas, zero descrições/valores nulos e ano 2022 em todas as linhas.

Despesas contratadas de 2018 usam [`tse_despesas_contratadas_2018.sql`](../../crates/refino/sql/silver/tse_despesas_contratadas_2018.sql) e publicam em `prestacao-contas/ano=2018/despesas-contratadas`. O contrato seleciona somente `_BRASIL`, que equivale às UFs mais `_BR` no inventário. As 1.724.053 ocorrências, incluindo 25.892 cópias exatas, são preservadas com manifesto, parte e ordinal de origem. Sequência e data ausentes ou sentinela tornam-se nulas; o valor é obrigatório e tipado como `Decimal128(18,2)`. A análise agregada do Bronze encontrou 7.555 sequências e datas ausentes e nenhum valor inválido. A fixture cobre cópias exatas, sentinelas e exclusão da parte estadual. Smoke integral no bucket local (2026-11-11) publicou 1.724.053 linhas em uma parte de 101.686.970 bytes, em 37,6 s no binário release. DuckDB confirmou 7.555 sequências e datas nulas, zero valores nulos, ordinais distintos em todas as linhas, ano 2018 em todo o conjunto e valores de R$ 0,00 a R$ 13.930.000,00.

Despesas contratadas de 2020 usam [`tse_despesas_contratadas_2020.sql`](../../crates/refino/sql/silver/tse_despesas_contratadas_2020.sql) e publicam em `prestacao-contas/ano=2020/despesas-contratadas`. O contrato seleciona somente `_BRASIL`, mantém as ocorrências repetidas do consolidado (55.321 cópias exatas identificadas no Bronze), converte sequência em `UInt32` anulável e data em `Date32` anulável, tipa o valor como `Decimal128(18,2)` obrigatório e preserva o original e a linhagem por manifesto, parte e ordinal. Sentinelas de sequência/data viram nulas; valores ausentes ou fora do formato validado impedem a publicação. A fixture inclui cópia exata e sentinelas. Smoke integral no bucket local (2026-11-11) publicou 4.107.531 linhas em uma parte de 460.048.948 bytes, em 101,8 s no binário release. DuckDB confirmou 218.785 sequências e datas nulas, uma descrição nula, zero valores nulos, ordinais distintos para todas as linhas e ano 2020 em todo o conjunto.

Despesas pagas de 2018 usam [`tse_despesas_pagas_2018.sql`](../../crates/refino/sql/silver/tse_despesas_pagas_2018.sql) e publicam em `prestacao-contas/ano=2018/despesas-pagas`. O contrato seleciona somente `_BRASIL`, equivalente às UFs mais `_BR` no inventário; preserva as 1.653.375 ocorrências e sua linhagem Bronze, mesmo sem cópias exatas no consolidado medido. Sequências de despesa e pagamento são `UInt32`, data é `Date32` e valor é `Decimal128(18,2)`, todos obrigatórios. A análise agregada do Bronze não encontrou sentinelas nem formatos inválidos nos campos tipados; valores variam de R$ 0,01 a R$ 6.000.000,00. A fixture cobre repetição sintética e exclusão da parte estadual. Smoke integral no bucket local (2026-11-11) publicou 1.653.375 linhas em uma parte de 102.624.333 bytes, em 21,8 s no binário release. DuckDB confirmou ordinais distintos para todas as linhas, zero nulos nos campos tipados e ano 2018 em todo o conjunto.

Despesas pagas de 2020 usam [`tse_despesas_pagas_2020.sql`](../../crates/refino/sql/silver/tse_despesas_pagas_2020.sql) e publicam em `prestacao-contas/ano=2020/despesas-pagas`. O contrato lê somente o consolidado `_BRASIL`, preserva cada pagamento e sua linhagem por manifesto, parte e ordinal, tipa sequências como `UInt32`, data como `Date32` e valor como `Decimal128(18,2)`. O inventário não encontrou repetições exatas; o contrato não remove ocorrências. A fixture verifica uma cópia exata sintética, tipagem dos campos e exclusão da parte estadual. Smoke integral no bucket local (2026-11-11) publicou 3.086.924 linhas em uma parte de 200.124.110 bytes, em 43,9 s no binário release. DuckDB confirmou a mesma contagem do Bronze, zero nulos nos campos tipados, ordinais distintos para todas as linhas e ano 2020 em todo o conjunto; valores pagos variam de R$ 0,01 a R$ 1.750.000,00.

Despesas pagas de 2022 usam [`tse_despesas_pagas_2022.sql`](../../crates/refino/sql/silver/tse_despesas_pagas_2022.sql), publicam em `prestacao-contas/ano=2022/despesas-pagas` e selecionam somente `_BRASIL`. O inventário não encontrou repetições exatas; o contrato ainda preserva cada ocorrência e a linhagem Bronze, sem deduplicação. Sequências viram `UInt32`, data `Date32` e valor `Decimal128(18,2)`; formatos inválidos impedem a publicação. A fixture confirma a preservação de uma repetição sintética e a exclusão da parte estadual. Smoke integral no bucket local publicou 2.411.963 linhas em uma parte de 145.638.474 bytes, em 33,6 s no binário release. A conferência agregada confirmou chaves candidatas distintas para todas as linhas, zero nulos e ano 2022 em todo o conjunto.

Use `todeolho refinar silver tse-prestacao-contas` para publicar as quatro famílias de 2016 (despesas e receitas de candidatos e partidos), as quatro famílias de 2018, 2020 e 2022 e as quatro tabelas contratadas de 2024. Os demais anos e tabelas aguardam contratos próprios. As tabelas que preservam variantes usam identidade técnica por versão, parte e ordinal. O CSV oficial de AC confirma que as colisões de despesas contratadas e receitas já estão na origem, conforme a pesquisa vinculada.

O ano de 2002 contém quatro conjuntos internos com outros nomes (`DespesaCandidato`, `ReceitaCandidato`, `DespesaComite`, `ReceitaComite`). A [validação específica de 2002](pesquisa-tse-2002.md) mediu schemas, tipos candidatos, nulos e duplicatas em cada um: todos têm cópias exatas e as combinações exploratórias de contexto/data/documento/valor colidem. O contrato candidato preserva cada linha com referência técnica à versão, parte e ordinal, sem deduplicação automática. Em 2012 há partes por sigla e consolidados com leiautes diferentes. Não aplicar a chave de 2024 retroativamente; a próxima validação deve separar outros anos e comparar consolidados com partes por UF.

A [validação de 2004 e 2006](pesquisa-tse-2004-2006.md) cobre oito tabelas internas, todas com cópias exatas. Os candidatos de 2004 não têm `SEQUENCIAL_CANDIDATO` e alguns cabeçalhos são expressões literais; em 2006, os candidatos mudam para nomes por extenso, enquanto os comitês mantêm outra convenção. Datas e valores preenchidos converteram nos formatos testados, mas a regra tipada precisa tratar ausências e impedir arredondamento de valores. Ainda não há chave de negócio validada para esses anos.

Em [2008](pesquisa-tse-2008.md), o manifesto tem quatro CSVs de contas e um TXT de leiaute que o Bronze também expôs como Parquet de uma linha. A Silver deve excluir `leiaute-2008` do catálogo tabular. Os quatro conjuntos de contas têm 7.124.045 linhas somadas, datas e valores preenchidos conversíveis nos formatos testados, mas alguns nomes físicos diferem do TXT de documentação e não há chave de lançamento validada.

O [inventário e a reconciliação de 2010 e 2012](pesquisa-tse-2010-2012.md) identificam seis famílias em cada ano. Em 2010, as 168 partes foram conferidas contra os membros do ZIP: a UF da coluna corresponde ao caminho original, embora o manifesto Bronze registre só o nome da família. Em 2012, os consolidados `_brasil` e as demais partes têm volumes e schemas diferentes; receitas partidárias por UF/`BR` têm 19 colunas posicionais contra 25 nomeadas em `_brasil`. A comparação completa de receitas e despesas de comitês encontrou linhas exclusivas em ambos os lados e colisões na chave candidata. Datas de receitas de candidatos e comitês exigem formatos distintos. Ainda não há chave de lançamento nem regra de consolidação para publicar essas famílias na Silver.

### Conjuntos da Câmara

Os sete conjuntos abaixo foram validados **apenas no ano de 2024**. Cada conjunto requer tabela silver própria, mesmo que o decodificador CSV do bronze use o nome genérico `votacoes`:

O leiaute oficial descreve `proposicoesAutores` como uma relação em que uma proposição pode aparecer uma vez por autor, então a chave precisa incluir a posição/ordem de assinatura. Para `votacoesVotos`, a origem descreve uma linha por voto de parlamentar e avisa que também pode haver registros individuais em algumas votações simbólicas. Portanto, votos não devem ser deduplicados nem descartados por tipo de votação. Os arquivos são atualizados diariamente; antes de transformar todos os anos, é necessário validar cada schema e as chaves candidatas contra os Parquets Bronze vigentes, não apenas contra o recorte de 2024. Consulte a [documentação oficial dos arquivos de proposições e votações](https://dadosabertos.camara.leg.br/swagger/api.html).

| Conjunto | Chave candidata | Linhas | Chaves distintas |
| --- | --- | ---: | ---: |
| `proposicoes` | `id` | 61.536 | 61.536 |
| `proposicoesAutores` | (`idProposicao`, `ordemAssinatura`) | 97.124 | 97.124 |
| `votacoes` | `id` | 10.373 | 10.373 |
| `votacoesVotos` | (`idVotacao`, `deputado_id`) | 116.400 | 116.400 |
| `votacoesObjetos` | (`idVotacao`, `proposicao_id`) | 60.950 | 60.950 |
| `votacoesProposicoes` | (`idVotacao`, `proposicao_id`) | 7.459 | 7.459 |
| `votacoesOrientacoes` | (`idVotacao`, `siglaBancada`) | 4.361 | 4.361 |

**Chaves rejeitadas ou ainda ambíguas:**

- Câmara despesas: `ideDocumento` tem 3.897.462 valores distintos em 5.235.506 linhas, com seis valores ausentes. Mesmo (`ano`, `ideDocumento`, `numParcela`, `nuDeputadoId`) tem só 4.078.766 combinações. Em 2024, as 232.918 linhas são distintas quando se exclui apenas a procedência; portanto a repetição de ID **não prova duplicação de registro**.
- Câmara tramitações: `idTramitacao` tem apenas 988 valores distintos em 1.879.478 linhas, com 174 ausentes. Mesmo (`uriProposicao`, `idTramitacao`) tem 1.871.298 combinações. Em 2024, há 112.979 linhas, 112.916 linhas distintas sem a procedência e 112.915 combinações de (`uriProposicao`, `idTramitacao`, `sequencia`, `dataHora`). Há duplicatas exatas e ao menos uma colisão semântica nessa chave composta; a política precisa distingui-las.
- Senado despesas: `COD_DOCUMENTO` tem 398.170 valores distintos em 398.172 linhas, com dois ausentes. A combinação com `ANO` ainda não é única (398.171 combinações), enquanto **todas as 398.172 linhas são distintas** sem a procedência. Revisar os documentos ausentes e não eliminar linhas por esse código.
- Senado processos: o glob local inclui um Parquet antigo com zero colunas, que o DuckDB não consegue ler; a validação de `id` sobre o conjunto completo ficou pendente. O decodificador atual já evita produzir esse formato em arrays vazios (PR #183), mas a amostra antiga permanece no armazenamento.
- A tabela `receitas_partidos_2012_MT` da prestação de contas do TSE (parte bronze `00150`, ano 2012) tinha uma data de registro como nome de coluna. Os 27 TXT dessa família com sufixo de duas letras não têm cabeçalho; o bronze corrigido publica colunas posicionais e preserva a primeira linha. A [pesquisa do leiaute de 2012](pesquisa-tse-2012.md) mostra que o `LEIAME.pdf` do ZIP não descreve as 19 posições e que o cabeçalho de 25 campos do consolidado não serve para mapeá-las. A chave silver continua pendente de leiaute oficial específico.

As evidências do código e das amostras locais ainda não cobrem todos os anos, partes e tabelas internas. Portanto as chaves e políticas de deduplicação da matriz continuam candidatas.

## Próxima validação da fase 0

Conferir amostras das 26 entradas do catálogo e todas as tabelas internas dos ZIPs, incluindo anos antigos e recentes; fixar chaves e schemas somente depois dessa conferência. A fase 1 (#186) implementa o mecanismo; as fases 2–4 (#187–#189) entregam os contratos específicos.
