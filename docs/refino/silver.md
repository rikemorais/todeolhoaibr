# Camada silver — contrato inicial

**Estado:** projeto da [fase 0 (#185)](https://github.com/rikemorais/todeolhoaibr/issues/185). Nenhuma tabela silver está materializada. A arquitetura proposta está no [ADR 0020](../adr/0020-contrato-da-camada-silver.md); responsabilidades das camadas estão no [ADR 0015](../adr/0015-arquitetura-de-dados-em-medalhao.md).

Silver lê somente o bronze vigente, aplica contratos por tabela e publica Parquet tipado no bucket `silver`. O contrato deve declarar grão, chave, tipos, nulos, duplicatas, qualidade e procedência. Chaves nesta matriz são **candidatas**: sua validação depende de amostras reais de cada tabela e de anos antigos/recentes. Identificadores, inclusive códigos IBGE, são texto; datas e valores devem ter tipos próprios. Nenhuma fonte sem município receberá um município inventado.

## Cobertura planejada do catálogo bronze

O [catálogo](../../crates/refino/src/catalogo.rs) tem 26 entradas. Uma entrada ZIP pode produzir várias tabelas internas; cada uma precisará de contrato próprio. `recurso` e `versão bronze` fazem parte da procedência, mas não substituem a chave de negócio.

| Entrada bronze | Família | Grão candidato da silver | Chave candidata / validação necessária |
| --- | --- | --- | --- |
| `camara-despesas` | Legislativo | despesa CEAP | documento e linha; validar IDs e correções |
| `camara-votacoes` | Legislativo | registro de um dos sete conjuntos anuais | contrato separado para `proposicoes`, `proposicoesAutores`, `votacoes`, `votacoesVotos`, `votacoesObjetos`, `votacoesProposicoes` e `votacoesOrientacoes`; chaves de 2024 abaixo |
| `camara-tramitacoes` | Legislativo | evento de tramitação | proposição e sequência/data; validar repetição |
| `camara-deputados` | Legislativo | deputado | ID oficial e vigência; validar histórico |
| `camara-partidos` | Legislativo | partido | ID oficial e vigência |
| `camara-legislaturas` | Legislativo | legislatura | ID oficial |
| `senado-despesas` | Legislativo | despesa CEAPS | documento/linha e ano; validar IDs |
| `senado-votacoes` | Legislativo | votação publicada | ID oficial e contexto; validar grão real |
| `senado-parlamentares` | Legislativo | parlamentar em legislatura | `IdentificacaoParlamentar.CodigoParlamentar` e `legislatura` no bronze local |
| `senado-processos` | Legislativo | processo | ID oficial e vigência |
| `senado-processos-detalhes` | Legislativo | detalhe de processo | `id` do processo (presente no fixture); eventos aninhados exigem decisão de tabela |
| `tse-candidaturas` | Eleitoral | candidatura por eleição | eleição, cargo, UF e identificador oficial |
| `tse-bens` | Eleitoral | bem declarado | candidatura e sequência do bem |
| `tse-prestacao-contas` | Eleitoral | lançamento por tabela interna | chave própria de receitas/despesas por ano; validar layouts |
| `tse-votacao-secao` | Eleitoral | voto agregado por seção | eleição, turno, seção, cargo e candidato; validar tabelas |
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

## Evidência disponível no repositório

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
| BCB `selic-mensal`, `data` | 483 | 483 | Sem nulos; confirmar nas demais séries. |
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
| TSE votação por seção 2014/AC, (`CD_ELEICAO`, `NR_TURNO`, `CD_MUNICIPIO`, `NR_ZONA`, `NR_SECAO`, `CD_CARGO`, `NR_VOTAVEL`) | 224.661 | 224.661 | Única no recorte. |
| TSE candidaturas 2014, uma parte, (`CD_ELEICAO`, `SQ_CANDIDATO`) | 879 | 879 | Única na parte; conferir entre UFs/partes. |
| TSE bens 2014, uma parte, (`CD_ELEICAO`, `SQ_CANDIDATO`, `NR_ORDEM_BEM_CANDIDATO`) | 3.588 | 3.588 | Única na parte; conferir entre UFs/partes. |

Os sete conjuntos da Câmara abaixo foram validados **apenas no ano de 2024**. Cada conjunto requer tabela silver própria, mesmo que o decodificador CSV do bronze use o nome genérico `votacoes`:

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
- Uma parte antiga de prestação de contas do TSE (2012) tem 22 colunas, das quais uma usa uma data de registro como nome; isso confirma cabeçalho inválido para um contrato estável. A chave dessa tabela fica suspensa até verificar o CSV de origem e o manifesto do bronze; não inferir schema a partir dessa parte.

As evidências do código e das amostras locais ainda não cobrem todos os anos, partes e tabelas internas. Portanto as chaves e políticas de deduplicação da matriz continuam candidatas.

## Próxima validação da fase 0

Conferir amostras das 26 entradas do catálogo e todas as tabelas internas dos ZIPs, incluindo anos antigos e recentes; fixar chaves e schemas somente depois dessa conferência. A fase 1 (#186) implementa o mecanismo; as fases 2–4 (#187–#189) entregam os contratos específicos.
