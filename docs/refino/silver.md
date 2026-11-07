# Camada silver — contrato inicial

**Estado:** implementação em andamento na [issue #188](https://github.com/rikemorais/todeolhoaibr/issues/188); a migração das transformações para SQL/DataFusion está em execução no [épico #236](https://github.com/rikemorais/todeolhoaibr/issues/236). BCB, agregados IBGE, Câmara e Senado já publicam tabelas Silver; consulte a cobertura abaixo. A arquitetura aceita está no [ADR 0020](../adr/0020-contrato-da-camada-silver.md); responsabilidades das camadas estão no [ADR 0015](../adr/0015-arquitetura-de-dados-em-medalhao.md).

Silver lê somente o bronze vigente, aplica contratos por tabela e publica Parquet tipado no bucket `silver`. O contrato deve declarar grão, chave, tipos, nulos, duplicatas, qualidade e procedência. Chaves nesta matriz são **candidatas**: sua validação depende de amostras reais de cada tabela e de anos antigos/recentes. Identificadores, inclusive códigos IBGE, são texto; datas e valores devem ter tipos próprios. Nenhuma fonte sem município receberá um município inventado.

## Contribuir com uma transformação SQL

Contratos SQL ficam versionados em `crates/refino/sql/silver/` e entram no binário com `include_str!`. A consulta lê a tabela `bronze`, preparada somente com as partes do manifesto fixado. Além das colunas Bronze, há três colunas internas de linhagem: `_tdoa_origem_manifesto`, `_tdoa_origem_parte` e `_tdoa_origem_ordinal`. A consulta deve projetar explicitamente as colunas Silver definidas pelo schema do contrato; preserve as colunas de origem necessárias para auditoria. Valores externos nunca são interpolados no SQL.

O adaptador `TarefaDeSilverSql` associa a consulta embutida, nome/versionamento da regra e schema de saída ao publicador comum. A execução transmite batches para Parquet temporário; não use coleta integral do resultado. O limite inicial é 512 MiB de memória, 2 GiB de spill, 100 GiB de entrada, uma partição por consulta e batches de 8.192 linhas. Esses limites são configuráveis por tarefa e devem ser medidos antes de ampliar uma família de fontes.

O smoke e benchmark sintético reproduzível do executor está em [`benchmark-datafusion.md`](benchmark-datafusion.md). Ele mede o pipeline SQL local com schema e linhagem Bronze e registra claramente quando os dados são sintéticos; a validação semântica com arquivos reais continua nos issues de migração por fonte.

## Cobertura planejada do catálogo bronze

O [catálogo](../../crates/refino/src/catalogo.rs) tem 26 entradas. Uma entrada ZIP pode produzir várias tabelas internas; cada uma precisará de contrato próprio. `recurso` e `versão bronze` fazem parte da procedência, mas não substituem a chave de negócio.

| Entrada bronze | Família | Grão candidato da silver | Chave candidata / validação necessária |
| --- | --- | --- | --- |
| `camara-despesas` | Legislativo | lançamento CEAP | `ideDocumento` é candidato, mas não único; preservar todas as linhas até definir chave |
| `camara-votacoes` | Legislativo | registro de um dos sete conjuntos anuais | contrato separado para `proposicoes`, `proposicoesAutores`, `votacoes`, `votacoesVotos`, `votacoesObjetos`, `votacoesProposicoes` e `votacoesOrientacoes`; chaves de 2024 abaixo |
| `camara-tramitacoes` | Legislativo | evento de tramitação | não há chave de negócio única comprovada; preservar repetições e usar proveniência do recurso/snapshot |
| `camara-deputados` | Legislativo | deputado em exercício na versão Bronze vigente | `id` oficial (513/513 distintos na amostra local; cada versão é uma fotografia) |
| `camara-partidos` | Legislativo | partido no cadastro observado | `id` oficial (21/21 distintos na amostra local; não prova unicidade histórica) |
| `camara-legislaturas` | Legislativo | período de atividade parlamentar | `id` sequencial (57/57 distintos na amostra local) |
| `senado-despesas` | Legislativo | lançamento CEAPS | `COD_DOCUMENTO` candidato, único quando presente na amostra; preservar linhas sem código |
| `senado-votacoes` | Legislativo | uma sessão de votação nominal | `codigoSessaoVotacao` (3.652/3.652 distintos no recorte local consultado); votos continuam aninhados |
| `senado-parlamentares` | Legislativo | parlamentar em legislatura | `IdentificacaoParlamentar.CodigoParlamentar` e `legislatura` no bronze local |
| `senado-processos` | Legislativo | um processo na listagem anual | `id` textual único no recurso; `ano` UInt16 do caminho |
| `senado-processos-detalhes` | Legislativo | um detalhe por processo em cada recurso anual | `id` textual; `ano` UInt16; demais campos preservados em `processo_original` |
| `tse-candidaturas` | Eleitoral | candidatura por eleição | ano, eleição, UF, unidade eleitoral, cargo, número e sequência do candidato; [validação histórica abaixo](#candidaturas-e-bens-do-tse-no-bronze-local-2026-11-05) |
| `tse-bens` | Eleitoral | bem declarado | ano, eleição, UF, unidade eleitoral, sequência do candidato e ordem do bem; alias da ordem muda por ano |
| `tse-prestacao-contas` | Eleitoral | lançamento por tabela interna | contrato por conjunto e ano; sequências de 2024 colidem no consolidado, conforme [validação abaixo](#prestacao-de-contas-do-tse-em-2024) |
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

### Referência técnica da linha bronze

O leitor [`LeitorDeParteBronze`](../../crates/refino/src/linhagem.rs) percorre uma parte Parquet em lotes, sem projeção ou filtro, e associa cada registro à chave do manifesto bronze, à chave da parte, ao nome da tabela de origem e ao ordinal da linha na parte (começando em 1). O ordinal continua entre lotes e não depende do tamanho do lote de leitura. Assim, duas linhas iguais no mesmo Parquet ainda têm referências distintas, e variantes com a mesma sequência oficial permanecem rastreáveis.

O [`SnapshotBronze`](../../crates/refino/src/snapshot_bronze.rs) lê o ponteiro e o manifesto vigentes, confere fonte/recurso e permite baixar somente partes listadas no manifesto. O download é em fluxo para arquivo temporário; tamanho e SHA-256 são verificados antes de abrir o leitor em lotes. Ponteiro ou manifesto inválido causam erro; ausência de ponteiro ou manifesto válido sem partes significa que não há dados Bronze a transformar.

Esse snapshot fixa a versão lida no início. A tarefa Silver ainda deverá conferir o ponteiro bronze antes de publicar sua própria versão, para não publicar resultado de uma entrada que já foi substituída. A referência da linha vale somente para a versão bronze selecionada e não substitui a chave de negócio. Nenhuma tabela Silver é publicada por esse leitor.

O [`EscritorParquetTipado`](../../crates/refino/src/escritor.rs) recebe o schema Arrow do contrato Silver e grava lotes `RecordBatch` em Parquet, inclusive para um arquivo temporário. Ele exige correspondência exata de nomes, tipos e nulabilidade em cada lote; data e decimal permanecem tipados. O teste de ida e volta cobre dois lotes, nulo e rejeição de escala decimal divergente.

O contrato `bcb.sgs.observacao.v1` é definido em [`bcb_sgs.sql`](../../crates/refino/sql/silver/bcb_sgs.sql): `data` em `Date32` e `valor` em `Decimal128(18,6)`, sem conversão por `f64`. A consulta exige data `DD/MM/AAAA` e valor decimal com ponto e até seis casas; dados inválidos ou nulos falham antes de publicar. O adaptador [`TarefaDeSilverBcb`](../../crates/refino/src/silver/tarefa_bcb.rs) usa `TarefaDeSilverSql` e o publicador comum, que mantém manifesto, ponteiro, idempotência e revalidação do snapshot Bronze. A regra `bcb-sgs-v2` reprocessa uma vez o que foi publicado pelo transformador Arrow anterior. A nulabilidade do schema publicado permanece a do contrato; nulos em campos obrigatórios são rejeitados na borda Arrow/Parquet. A CLI aceita `todeolho refinar silver bcb` e `todeolho refinar silver --todas --intervalo 3600`. A execução Silver aparece no painel com o selo azul.

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

O contrato `senado.parlamentar.legislatura.v1` extrai `CodigoParlamentar` do JSON compacto de `IdentificacaoParlamentar`, tipa `legislatura` como `UInt16` a partir do recurso `parlamentares/legislatura=NN` e preserva os objetos de identificação e mandato como JSON. A chave candidata (`CodigoParlamentar`, `legislatura`) foi validada no Bronze local; o contrato preserva o mandato aninhado sem explodi-lo em linhas. A [OpenAPI oficial do Senado](https://legis.senado.leg.br/dadosabertos/v3/api-docs) documenta a listagem por legislatura. A CLI oferece `todeolho refinar silver senado-parlamentares`; a fonte também entra em `--todas` e na reconciliação periódica.

O contrato `senado.processo.v1` publica uma linha por matéria em cada listagem anual, tipa `id` como texto e `ano` como `UInt16`, e mantém todos os campos da API em `processo_original` JSON. A listagem anual é uma fotografia do catálogo consultado naquele ano, não um histórico de versões da matéria. IDs foram únicos e não vazios nos snapshots locais de 2010 (2.965 linhas), 2023 (5.432) e 2024 (4.947); essa validação é por recurso anual. A CLI oferece `todeolho refinar silver senado-processos`; a fonte entra em `--todas` e na reconciliação periódica.

O contrato `senado.processo.detalhe.v1` publica uma linha por processo no recurso anual, com `id` como texto, `ano` como `UInt16` e os campos Bronze preservados em `processo_original` JSON. A tarefa valida IDs não vazios e únicos no recurso, sem explodir despachos, classificações ou outros objetos aninhados. No Bronze local consultado, 163.156 linhas tinham 163.156 IDs distintos e anos válidos. A CLI oferece `todeolho refinar silver senado-processos-detalhes`; a fonte entra em `--todas` e na reconciliação periódica.

O leitor Silver também aceita partes Bronze legadas cujo Parquet tem schema raiz vazio e nenhuma linha: elas produzem uma parte Silver vazia usando o schema declarado pelo contrato. Um arquivo sem colunas que declare linhas continua sendo erro.

O contrato `senado.votacao.sessao.v1` valida `codigoSessaoVotacao` por recurso anual e preserva cada registro do Bronze, incluindo a coluna `votos` como JSON compacto aninhado. A chave teve 3.652 valores distintos em 3.652 linhas no recorte local consultado; anos diferentes ainda exigem smoke test. O schema completo continua no formato texto do Bronze. O contrato complementar `senado.voto.nominal.v1` publica uma linha por voto em `votos/ano=AAAA`, mantendo `codigo_sessao_votacao` e `data_sessao`, o ordinal original `ordem_voto`, os campos disponíveis do parlamentar (código, nome, partido e UF), o voto e o objeto integral em `voto_original` como JSON compacto. A chave técnica da relação é (`codigo_sessao_votacao`, `ordem_voto`): ela preserva ocorrências repetidas sem pressupor unicidade do parlamentar na sessão. Sessões sem votos continuam na relação de sessões e não geram linhas na relação de votos; identificadores parlamentares ausentes ou inválidos são rejeitados. O decodificador reconhece os nomes atuais e o envelope legado `Votos.VotoParlamentar`; todos os campos publicados permanecem texto, exceto o ordinal UInt32. A documentação oficial descreve os campos da votação e dos votos parlamentares como tabelas relacionadas ([serviço de votações nominais](https://www12.senado.leg.br/dados-abertos/legislativo/plenario/votacoes-nominais/info/webservice-de-votacoes-nominais-por-data-da-sessao-plenaria)). A CLI `todeolho refinar silver senado-votacoes` publica as duas relações e segue incluída em `--todas` e na reconciliação periódica.

O contrato `camara.partido.v1` valida `id` como identificador não nulo e não vazio e preserva `id`, `sigla`, `nome` e `uri` como texto. O ID teve 21 valores distintos nas 21 linhas da amostra local descrita acima; isso é evidência apenas para o cadastro observado, não para histórico ou futuras versões. A projeção e a validação de unicidade agora ficam no contrato SQL `camara_partidos.sql`, executado pelo publicador DataFusion. A regra v2 reprocessa os snapshots existentes uma vez. A tarefa acompanha o ponteiro Bronze e é idempotente por versão e regra. A CLI oferece `todeolho refinar silver camara-partidos`; a fonte também entra em `--todas` e na reconciliação periódica.

O contrato `camara.legislatura.periodo.v1` tipa o ID sequencial como `UInt16`, as datas `dataInicio` e `dataFim` como `Date32` e o ano `dataEleicao` como `UInt16`; preserva `uri` como texto. Rejeita IDs inválidos ou duplicados, datas fora do formato ISO e intervalos invertidos. A transformação agora está no contrato SQL `camara_legislaturas.sql`, que usa `arrow_try_cast` para produzir os tipos unsigned do contrato; a regra v2 reprocessa os snapshots existentes uma vez. A documentação oficial descreve os períodos parlamentares como sequenciais desde a primeira legislatura e lista início, fim e ano eleitoral ([API de legislaturas](https://dadosabertos.camara.leg.br/api/v2/api-docs)). O conjunto observado teve 57 IDs distintos em 57 linhas. A CLI oferece `todeolho refinar silver camara-legislaturas`; a fonte participa de `--todas` e da reconciliação periódica.

O contrato `camara.deputado.exercicio.v1` trata cada versão Bronze como a fotografia vigente dos deputados em exercício, sem inferir histórico a partir de uma listagem sem intervalo temporal. Preserva `id`, URIs, nome, partido, UF, legislatura, foto e e-mail; identificadores e atributos categóricos permanecem texto. `id` deve ser numérico, obrigatório e único na fotografia. A transformação está no contrato SQL `camara_deputados.sql`, e a regra v2 reprocessa os snapshots existentes uma vez. A lista da Câmara teve 513 IDs distintos nas 513 linhas observadas. A [API oficial](https://dadosabertos.camara.leg.br/api/v2/api-docs) descreve o endpoint de deputados e os filtros temporais; a CLI oferece `todeolho refinar silver camara-deputados`, incluído em `--todas` e na reconciliação periódica.

Os sete contratos da fonte `camara-votacoes` validam a chave candidata de cada conjunto por recurso anual e preservam todas as colunas e linhas do Bronze, com o schema de origem (texto). Seis regras usam versão v1; `votacoesVotos` usa `camara-votacoes-votos-v2`. Não fazem deduplicação nem cruzamento entre tabelas. Em `votacoesVotos`, linhas com chave candidata incompleta são preservadas e não participam da verificação de duplicidade. No snapshot local de 2004 havia uma linha sem `deputado_id`; ela foi mantida na publicação. Os Parquets Silver são independentes por conjunto e ano; `--todas` percorre os recursos Bronze presentes, e a pré-checagem inclui snapshot e versão da regra. A CLI oferece `todeolho refinar silver camara-votacoes`. A matriz de unicidade disponível foi medida somente em 2024: os anos anteriores ainda precisam de smoke test contra Bronze vigente antes de serem considerados cobertos por evidência.

O console DuckDB cria views `bronze_*` e `silver_*` a partir dos ponteiros vigentes e dos manifestos correspondentes. As views Silver incluem apenas Parquets publicados pela Silver; execuções antigas permanecem fora do glob.

## Evidência disponível no repositório

### Despesas da Câmara (CEAP)

O contrato `camara.despesa.ceap.v2` publica uma linha por lançamento anual, tipa o ano do caminho como `UInt16`, datas de emissão/restituição como `Date32` e os quatro valores monetários como `Decimal128(18,6)`. A precisão de seis casas é necessária para preservar valores históricos de 2013–2015; no Bronze local, nenhuma linha supera 12 dígitos inteiros ou seis decimais. Identificadores permanecem texto. Campos ausentes ou em branco nesses campos opcionais viram nulos; valor preenchido fora do formato esperado interrompe a publicação. `registro_original` preserva todos os campos do Bronze, inclusive campos que ainda não têm projeção tipada. O transformador não deduplica: `ideDocumento` colide no conjunto histórico e não é chave única comprovada. A CLI `todeolho refinar silver camara-despesas` aceita um recurso anual, participa de `--todas` e da reconciliação Silver.

### Tramitações da Câmara

O contrato `camara.tramitacao.evento.v1` publica cada linha do recurso anual e tipa `ano` como `UInt16`, `sequencia` como `UInt32` quando presente e `dataHora` como `Timestamp(Microsecond)`. Identificadores, códigos e números permanecem texto; `registro_original` retém todos os campos Bronze. A amostra local tinha 1.879.478 linhas: `idTramitacao` possuía apenas 988 valores distintos e faltava em 174 linhas; o par `uriProposicao` + `sequencia` se repetia, inclusive com órgãos/despachos diferentes e com cópias exatas. Por isso nenhuma dessas combinações é tratada como chave única, e nenhuma linha é rejeitada ou deduplicada por repetição. A proveniência do recurso e do snapshot Bronze identifica o conjunto processado. A CLI `todeolho refinar silver camara-tramitacoes` publica os anos individualmente e integra `--todas` e a reconciliação Silver.

### Candidaturas do TSE

O contrato `tse.candidatura.registro.v1` publica uma linha por registro do consolidado nacional `BRASIL` em cada recurso eleitoral anual. As partes por UF do mesmo ZIP não são somadas ao consolidado. `ano_eleicao` é `UInt16`; `codigo_eleicao`, `uf`, `unidade_eleitoral`, `codigo_cargo`, `numero_candidato` e `sequencia_candidato` permanecem texto para preservar códigos e zeros à esquerda. A chave candidata é a combinação completa desses campos com o ano do recurso; ela é validada entre lotes e duplicatas interrompem a publicação. A análise Bronze mediu unicidade dessa chave nos 17 anos eleitorais locais, de 1994 a 2026; a chave curta eleição + sequência colide em eleições antigas.

`data_eleicao` e `data_nascimento` são `Date32`; `idade_data_posse` é `UInt8`. Datas não interpretáveis ou fora da faixa plausível e sentinelas de idade são tipadas como nulas, sem remover a linha. `registro_original` conserva todos os campos e nomes publicados naquele ano, inclusive os aliases históricos (por exemplo `NM_EMAIL`/`DS_EMAIL`), sem projetá-los como colunas canônicas antes de uma regra de domínio. A regra `tse-candidatura-registro-v1` acompanha a reconciliação de cada ano. Use `todeolho refinar silver tse-candidaturas` para processar os recursos disponíveis; a fonte também participa de `silver --todas`.

Smoke local nos 17 consolidados Bronze (1994–2026): 3.703.307 linhas de entrada e saída, sem linhas removidas, 2.428 datas da eleição, 121.623 datas de nascimento e 1.464.963 idades tipadas como nulas (inclui campos ausentes e sentinelas), e zero data da eleição fora da faixa tipada. A chave completa foi validada durante o processamento de cada lote. A segunda reconciliação não republicou nenhum ano, e o modo periódico não registrou execução no painel quando todos já estavam em dia. Com concorrência 2, a execução levou 7min41s; o recurso mais lento foi 2012, em 143,3s. O fluxo limita cada lote a 65.536 registros; esta execução não mediu o pico de RSS, que permanece pendente como evidência operacional da issue #189.

### Bens declarados do TSE

O contrato `tse.candidatura.bem.v1` publica uma linha por bem no consolidado nacional `BRASIL` de cada eleição, sem somar as partes por UF. A chave composta é (`ano_eleicao`, `codigo_eleicao`, `uf`, `unidade_eleitoral`, `sequencia_candidato`, `ordem_bem`), validada entre lotes. A ordem é lida de `NR_ORDEM_CANDIDATO` nos layouts de 2006–2012 e 2016 e de `NR_ORDEM_BEM_CANDIDATO` em 2014 e 2018–2026; não se infere o alias por uma regra monotônica. A chave eleitoral do bem também permite vinculá-lo à candidatura de mesmo ano, eleição, UF, unidade eleitoral e sequência do candidato.

`ano_eleicao` é `UInt16`, `ordem_bem` é `UInt32`, as datas são `Date32` e `valor_bem` é `Decimal128(18,2)`. Valores são convertidos por aritmética decimal exata a partir da vírgula brasileira. Datas não interpretáveis ou fora da faixa plausível viram nulas sem remover a linha, e `registro_original` preserva todos os campos e aliases de origem; isso é necessário, por exemplo, para valores de última atualização antigos que não são datas válidas. Smoke local nos 11 consolidados (2006–2026): 5.101.128 linhas de entrada e saída, zero valores ou ordens nulos/inválidos, 8.967 datas de eleição nulas e 4.041.301 atualizações nulas; nenhuma linha foi removida. Um join Silver por ano encontrou exatamente uma candidatura para cada bem (zero bens sem correspondência). A segunda passada reconheceu os 11 recursos como atualizados e não os republicou; a primeira execução levou 4min52s com concorrência 2, sem medição de pico de RSS (evidência operacional ainda pendente na issue #189). Use `todeolho refinar silver tse-bens`; a fonte também integra `silver --todas` e sua reconciliação periódica.

### Votação por seção do TSE

O contrato `tse.votacao.secao.v1` publica uma linha por (`ano_eleicao`, `codigo_eleicao`, `uf`, `turno`, `codigo_municipio`, `zona`, `secao`, `codigo_cargo`, `numero_votavel`) a partir de cada recurso `votacao_secao/ano=AAAA/uf=UF`. Ano e turno são inteiros tipados, votos são `UInt32`, data de eleição é `Date32`; os códigos permanecem texto e todos os campos de origem são preservados em `registro_original`. A chave é validada entre lotes e uma repetição interrompe a publicação.

Os recursos `BR` e estaduais são complementares: o primeiro contém votos presidenciais e os segundos cargos estaduais. A tarefa mantém ambos, validando que cada linha estadual pertence à UF do recurso. Os recursos 1998/BR e 2010/VT e 2010/ZZ são excluídos porque a Bronze atual não recupera seus cabeçalhos e a chave padrão não está disponível. O TSE informa que os resultados de 1998 são incompletos ([catálogo oficial](https://dadosabertos.tse.jus.br/dataset/resultados-1998)); esses três recursos precisam de trabalho separado na decodificação Bronze antes de entrarem no contrato. O restante do histórico precisa ser monitorado contra mudanças nos layouts publicados.

Execute explicitamente com `todeolho refinar silver tse-votacao-secao`. A coleção local consultada ocupa cerca de 16,4 GB, então não participa de `silver --todas` ou da sua reconciliação periódica. Para smoke parcial, use `--filtro ano=2024 --filtro uf=AC,RO`; ambos os filtros são opcionais e repetíveis, e os valores aceitam listas separadas por vírgula. Votos por seção anteriores a 1998 não são publicados pelo TSE. A cobertura da chave foi validada no Bronze em AC e RO de 1998–2026. No smoke Silver 2024, AC e RO publicaram 156.259 e 349.871 linhas respectivamente, contagens iguais às partes Bronze e sem duplicatas de chave composta. Três reconciliações seguintes reconheceram ambos recursos como em dia.

### Despesas do Senado (CEAPS)

O contrato `senado.despesa.ceaps.v1` tipa `ANO` do recurso como `UInt16`, `MES` como `UInt8`, `DATA` como `Date32` e `VALOR_REEMBOLSADO` como `Decimal128(18,2)`. CNPJ/CPF, documento e `COD_DOCUMENTO` continuam texto para preservar zeros à esquerda. O parser usa decimal com vírgula sem `f64`, aceita frações de zero a duas casas, valores como `,95` e quebras CR/LF embutidas no valor histórico de 2013. No Bronze local havia 398.172 linhas de 2008 a 2026; `COD_DOCUMENTO` foi único em 398.170 linhas e estava ausente em duas. O código é validado quando preenchido; registros sem código não são removidos nem deduplicados. Dois registros de 2015 têm aspas malformadas no campo `DOCUMENTO` da origem CSV, o que desloca colunas no Bronze: preservamos essas linhas e os campos Bronze em `registro_original`, mas deixamos data e valor tipados nulos para não interpretar texto deslocado como despesa. Para as demais linhas, a Silver deixa `DATA` tipada nula quando o ano é anterior a 1900 ou posterior ao ano de competência + 1, preservando o texto de origem; essa faixa cobre as 75 datas parses fora dos limites encontradas na amostra Bronze local. Valores negativos permanecem com o sinal recebido. A regra `senado-despesa-ceaps-v3` registra essa política para que recursos já publicados sejam reconciliados. A CLI `todeolho refinar silver senado-despesas` publica cada ano e participa de `--todas` e da reconciliação Silver.

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

Nas 56 chaves repetidas de despesas pagas, a comparação exata encontrou 135 linhas e 56 linhas distintas: as 79 excedentes são cópias idênticas no consolidado local. A tarefa Silver implementada para `despesas_pagas_candidatos` de 2024 usa (`AA_ELEICAO`, `CD_ELEICAO`, `SQ_PRESTADOR_CONTAS`, `SQ_DESPESA`, `SQ_PARCELAMENTO_DESPESA`), remove somente registros integrais iguais, contabiliza as cópias no log e falha se uma chave tiver conteúdo diferente. `SQ_PRESTADOR_CONTAS` permanece texto porque seus valores não cabem em `UInt32`; despesas pagas, data e valor são tipados. Smoke na parte `BRASIL`: 3.719.729 linhas de entrada, 3.719.650 de saída, 79 cópias exatas removidas, zero nulos em data/valor e zero chaves repetidas. A execução no perfil de desenvolvimento levou 525,5 s; o maior RSS amostrado foi cerca de 1,07 GB. A reconciliação seguinte reconheceu o recurso em dia.

Use `todeolho refinar silver tse-prestacao-contas` para publicar essa tabela. A cobertura permanece limitada a despesas pagas de 2024; despesas contratadas, receitas, doador originário e os demais anos aguardam contratos próprios. Despesas contratadas, receitas e doador originário devem preservar variantes até que a identidade de cada linha esteja definida. O leitor já fornece identidade técnica por versão, parte e ordinal. O CSV oficial de AC confirma que as colisões de despesas contratadas e receitas já estão na origem, conforme a pesquisa vinculada.

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
