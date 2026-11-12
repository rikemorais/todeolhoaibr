# Fonte `ibge-renda-per-capita` — Renda domiciliar per capita dos municípios

Rendimento domiciliar per capita por município, dos **Censos 2010 e 2022**, via API SIDRA do IBGE — fornece a espinha dorsal socioeconômica que alimenta o join `município + ano` com os dados eleitorais. Implementa a [issue #29](https://github.com/rikemorais/todeolhoaibr/issues/29) (Fase 1: municipal).

## O que é ingerido

Três variáveis por edição do Censo, no nível **municipal** (`N6`, ~5.570 municípios), da API SIDRA (`apisidra.ibge.gov.br/values`):

| Censo | Tabela SIDRA | Variáveis | Recurso na Raw |
| --- | --- | --- | --- |
| **2010** | 3578 | 2012 (média), 2013 (mediana), 2035 (moradores) | `renda-per-capita/censo=2010` |
| **2022** | 10295 | 13431 (média), 13534 (mediana), 13604 (moradores) | `renda-per-capita/censo=2022` |

!!! note "Censos 1991 e 2000"
    Os censos de 1991 e 2000 **não** estão disponíveis na API SIDRA no nível municipal (`N6`). A issue prevê série comparável, mas os dados anteriores a 2010 só existem em microdados ou publicações tabulares, fora do escopo desta Fase 1.

!!! note "Classificações de 2022"
    A tabela 10295 exige filtros de classificação para obter totais: sexo = Total (`c2/6794`), faixa etária = Total (`c287/95253`), cor/raça = Total (`c86/95251`).

## Layout na Raw

```text
raw/ibge/renda-per-capita/censo=2010/_ultima_versao.json
raw/ibge/renda-per-capita/censo=2010/data_ingestao=AAAA-MM-DD/execucao=<id>/parte-00001.json
raw/ibge/renda-per-capita/censo=2022/...
```

## API diferente das existentes

A API SIDRA (`/values`) devolve um **array JSON flat** onde a primeira linha é o cabeçalho com descrições e as demais são dados com colunas codificadas (`NC`, `NN`, `V`, `D1C`, `D1N`, `D2C`, `D2N`, `MC`, `MN`...). Isso é diferente da API de agregados (v3) usada por `ibge-ipca`, `ibge-pib-municipios`, etc. O Bronze usa `DecodificadorJson` (array de objetos); o **Silver SQL filtra o cabeçalho** (`NC <> 'Nível Territorial (Código)'`).

## Fonte separada, `sob_demanda`

É uma **fonte própria `ibge-renda-per-capita` com `sob_demanda = true`**, separada da [`ibge`](ibge.md) (malha territorial e indicadores macro). São apenas duas tarefas (~2 MB cada), mas mantê-la fora do `executar --todas` segue o padrão de dados censitários sob demanda. O dado mora em `raw/ibge/`.

## Como rodar

```bash
# Os dois censos
todeolho executar ibge-renda-per-capita

# Bronze
todeolho refinar bronze ibge-renda-per-capita

# Silver
todeolho refinar silver ibge-renda-per-capita
```

## Silver

O contrato `ibge-renda-per-capita-v1` produz uma linha por (município, variável, censo) com:

- `municipio_id` / `municipio_nome`
- `variavel_id` / `variavel_nome`
- `unidade`
- `ano_censo`
- `valor_original` / `valor_decimal` (Decimal(38,9))
- colunas de linhagem `_tdoa_origem_*`

Sentinelas de valor (`..`, `-`, `X`) ficam como `valor_original` mas o `valor_decimal` fica nulo.

## Idempotência

Como as demais fontes de API, a pré-checagem é por estado na Raw: um censo já ingerido e íntegro não é rebaixado ([ADR 0012](../adr/0012-pre-checagem-idempotente.md)). Um limite de 2 req/s em `apisidra.ibge.gov.br` evita sufocá-lo.

## Fase 2: Setores censitários (futura)

Os dados por setor censitário do Censo 2022 estão disponíveis em `ftp.ibge.gov.br` (via HTTPS). Uma PR futura adicionará `ibge-renda-per-capita-setores` usando `TarefaDeArquivo` para downloads streaming, com tratamento da proteção estatística de setores pequenos.
