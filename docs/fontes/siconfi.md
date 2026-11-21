# Fonte `siconfi` — dados fiscais de estados e municípios (Tesouro Nacional)

Dados fiscais de **todos os entes federativos** (estados, municípios e União) publicados pelo **Tesouro Nacional** via API SICONFI Data Lake. Cobre as três peças obrigatórias da Lei de Responsabilidade Fiscal: DCA (contas anuais/FINBRA), RREO (execução orçamentária) e RGF (gestão fiscal). Implementa a [issue #55](https://github.com/rikemorais/todeolhoaibr/issues/55).

## O que é ingerido

Quatro fontes independentes, todas sobre o mesmo `FONTE_RAW = "siconfi"`:

| Fonte | O que é | Sob demanda | Partição |
| --- | --- | --- | --- |
| `siconfi-entes` | Cadastro dos ~5.598 entes federativos | Não | — |
| `siconfi-dca` | Declaração de Contas Anuais (FINBRA) | Sim | UF × ano |
| `siconfi-rreo` | Relatório Resumido de Execução Orçamentária | Sim | UF × ano × bimestre |
| `siconfi-rgf` | Relatório de Gestão Fiscal | Sim | UF × ano × quadrimestre |

### API

Base: `https://apidatalake.tesouro.gov.br/ords/siconfi/tt/`

A API é pública (sem autenticação), baseada em Oracle ORDS. As respostas usam o envelope `{items, hasMore, limit, offset, count, links}`. Não há endpoint bulk — cada demonstrativo exige uma consulta por entidade (`id_ente`).

### Estratégia de paginação

As fontes DCA/RREO/RGF reutilizam `EstrategiaDePaginacao` + `TarefaDeIngestaoHttp::paginada` seguindo o padrão de `processos_detalhes` do Senado:

1. A primeira URL de cada tarefa busca `/entes` (o cadastro de ~5.598 entidades).
2. O `proxima_url` parseia a listagem, filtra pela UF da tarefa e gera as URLs de cada entidade para o demonstrativo específico.
3. Cada URL gerada é baixada como uma parte independente no raw.

Para o RGF, cada entidade gera **5 URLs** (uma por poder: Executivo, Legislativo, Judiciário, Ministério Público e Defensoria Pública).

## Layout na Raw

```text
raw/siconfi/entes/data_ingestao=.../execucao=.../parte-00001.json
raw/siconfi/dca/uf=SP/ano=2024/data_ingestao=.../execucao=.../parte-00001.json  (entes)
raw/siconfi/dca/uf=SP/ano=2024/data_ingestao=.../execucao=.../parte-00002.json  (entidade 1)
raw/siconfi/dca/uf=SP/ano=2024/data_ingestao=.../execucao=.../parte-00003.json  (entidade 2)
...
```

## Como rodar

As fontes DCA/RREO/RGF são `sob_demanda` — precisam de `--filtro` para executar:

```bash
# Cadastro de entes (leve, entra no --todas)
todeolho executar siconfi-entes

# DCA de Tocantins, exercício 2024
todeolho executar siconfi-dca --filtro uf=TO,ano=2024

# RREO de São Paulo, 3o bimestre de 2024
todeolho executar siconfi-rreo --filtro uf=SP,ano=2024,bimestre=3

# RGF de Minas Gerais, 1o quadrimestre de 2024
todeolho executar siconfi-rgf --filtro uf=MG,ano=2024,quadrimestre=1
```

## Idempotência

Cada recurso `dca/uf=SP/ano=2024` cobre um par (UF, ano) único. Dentro do recurso, cada entidade tem sua parte. Re-execuções sobrescrevem via ponteiro `_ultima_versao.json`. O `ETag` de `TarefaDeIngestaoHttp` permite GET condicional quando disponível.

## Refino (Bronze)

- **`siconfi-entes`**: decodifica com `DecodificadorJson::com_caminho("entes", &["items"])`.
- **`siconfi-dca/rreo/rgf`**: usa `DecodificadorSiconfi`, que **pula a primeira parte** (listagem de entes) e agrega os `items` de todas as partes seguintes em uma tabela Parquet única.

## Etiquetas disponíveis para filtro

| Etiqueta | Fontes | Valores |
| --- | --- | --- |
| `uf` | DCA, RREO, RGF | AC, AL, AM, ..., TO (27 UFs) |
| `ano` | DCA, RREO, RGF | 2015 em diante |
| `bimestre` | RREO | 1 a 6 |
| `quadrimestre` | RGF | 1 a 3 |
