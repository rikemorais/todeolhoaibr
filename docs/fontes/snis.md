# Fonte `snis` — saneamento por município (SINISA / ex-SNIS)

Dados de **saneamento básico por município** publicados pelo **Ministério das Cidades** via SINISA (Sistema Nacional de Informações sobre Saneamento, antigo SNIS). Cobre abastecimento de água, esgotamento sanitário, resíduos sólidos e águas pluviais. Implementa a [issue #49](https://github.com/rikemorais/todeolhoaibr/issues/49).

## O que é ingerido

Quatro fontes independentes, todas sobre o mesmo `FONTE_RAW = "snis"`:

| Fonte | O que é | Sob demanda | Partição |
| --- | --- | --- | --- |
| `snis-agua` | Abastecimento de água | Sim | ano |
| `snis-esgoto` | Esgotamento sanitário | Sim | ano |
| `snis-residuos` | Resíduos sólidos | Sim | ano |
| `snis-pluviais` | Águas pluviais (drenagem urbana) | Sim | ano |

### Origem dos dados

Não há API pública — os dados são publicados como **planilhas XLSX dentro de ZIPs** no portal gov.br. O SNIS foi substituído pelo SINISA em 2023 (Lei 14.026/2020, Marco Legal do Saneamento). As URLs são extraídas do catálogo do projeto [saneabr](https://github.com/theoadepaula/saneabr).

### Escopo atual

SINISA **2023–2024**, somente arquivos em formato ZIP:

| Módulo | 2023 | 2024 |
| --- | --- | --- |
| Água | ZIP (19 MB, compartilhado com esgoto) | ZIP (19 MB) |
| Esgoto | ZIP (mesmo arquivo de água 2023) | ZIP (8.6 MB) |
| Resíduos | ~~RAR (16 MB)~~ — fora do escopo | ZIP (18 MB) |
| Pluviais | ~~RAR (7 MB)~~ — fora do escopo | ZIP (9.5 MB) |

Não ingeridos: dados SNIS históricos (formato XLS), Gestão Municipal (XLSX solto), e arquivos RAR (sem suporte no codebase).

### Formato XLSX (Base Municipal)

Cada ZIP contém subpastas (Base Municipal, Consolidado UF, Locais+Regionais). O decodificador filtra apenas os XLSX em subpastas "Base Municipal". Cada planilha tem:

- Linhas 1–7: preâmbulo (títulos ministeriais, data de publicação)
- Linha 8: categorias agrupadas (Dados Gerais, Atendimento, …)
- Linha 9: nomes legíveis das colunas
- Linha 10: unidades (Percentual, m³/ano, …)
- **Linha 11**: códigos máquina (`cod_IBGE`, `CAD0002`, `IAG0001`, …) — usados como cabeçalho do Bronze
- Linha 12+: dados (~5.150 municípios)

Valores podem ser numéricos ou textuais ("Não calculado (divisão por zero)").

## Layout na Raw

```text
raw/snis/agua/ano=2023/data_ingestao=.../execucao=.../parte-00001.zip
raw/snis/agua/ano=2024/data_ingestao=.../execucao=.../parte-00001.zip
raw/snis/esgoto/ano=2023/data_ingestao=.../execucao=.../parte-00001.zip
raw/snis/esgoto/ano=2024/data_ingestao=.../execucao=.../parte-00001.zip
raw/snis/residuos/ano=2024/data_ingestao=.../execucao=.../parte-00001.zip
raw/snis/pluviais/ano=2024/data_ingestao=.../execucao=.../parte-00001.zip
```

## Como rodar

Todas as fontes são `sob_demanda` — precisam de `--filtro` ou nome explícito:

```bash
# Água, exercício 2024
todeolho executar snis-agua --filtro ano=2024

# Esgoto, todos os anos disponíveis
todeolho executar snis-esgoto

# Resíduos sólidos
todeolho executar snis-residuos

# Águas pluviais
todeolho executar snis-pluviais
```

## Idempotência

Cada recurso `agua/ano=2024` cobre um par (módulo, ano) único. `TarefaDeArquivo` usa conditional GET (ETag) para não rebaixar o que não mudou na origem. Re-execuções sobrescrevem via ponteiro `_ultima_versao.json`.

## Refino (Bronze)

`DecodificadorSnis` abre o ZIP, filtra XLSX em subpastas "Base Municipal", e para cada planilha:

1. Localiza a linha-cabeçalho (primeira com `cod_IBGE` na coluna A)
2. Usa os códigos máquina dessa linha como nomes de coluna
3. Converte todas as linhas de dados para strings
4. Adiciona coluna `_planilha` com o nome do arquivo de origem

Resultado: um Parquet por planilha "Base Municipal" encontrada no ZIP.

## Etiquetas disponíveis para filtro

| Etiqueta | Fontes | Valores |
| --- | --- | --- |
| `ano` | Todas | 2023, 2024 |
