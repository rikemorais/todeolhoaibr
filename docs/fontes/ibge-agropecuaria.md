# IBGE — Pesquisas Agropecuárias (SIDRA)

Fonte: **apisidra.ibge.gov.br** · Formato: JSON · Atualização: anual (PAM,
PPM, PEVS) / mensal (LSPA) / decenal (Censo) · Licença: dados abertos

## O que é ingerido

Cinco pesquisas do sistema SIDRA do IBGE, todas no nível municipal (`N6`):

| Pesquisa | Tabela | Período | Variáveis |
|----------|--------|---------|-----------|
| **PAM** (Produção Agrícola Municipal) | 5457 | 1974–2025 | área plantada/colhida, quantidade, valor e rendimento (total agregado) |
| **PPM** (Pecuária Municipal) | 3939 | 1974–2025 | efetivo dos rebanhos (total agregado) |
| **LSPA** (Levantamento Sistemático da Produção Agrícola) | 6588 | 2006–2026 | área plantada/colhida, produção, rendimento (mensal) |
| **PEVS** (Extração Vegetal e Silvicultura) | 289 | 1986–2025 | quantidade e valor da produção extrativa |
| **Censo Agropecuário** | 6881 | 2017 | estrutura fundiária, uso da terra, condição do produtor |

**Total: 166 tarefas (~1 GB), sob demanda.**

## API SIDRA

Padrão de URL:
```
https://apisidra.ibge.gov.br/values/t/{tabela}/n6/all/v/all/p/{periodo}[/{classificação}]
```

- Retorna JSON (array de objetos), sem autenticação
- `n6/all` = todos os municípios (~5.570)
- `v/all` = todas as variáveis da tabela
- Para PAM: `/c782/0` (classificação "Produto das lavouras", total)
- Para PPM: `/c79/0` (classificação "Tipo de rebanho", total)
- Para LSPA: períodos mensais (`{ano}01,{ano}02,...,{ano}12`)

## Sistemas de origem

Dois sistemas complementam o SIDRA:

- **SIFISC** — sistema antigo de fichas; dados mais detalhados no nível
  municipal disponíveis apenas via FTP para anos antigos
- **SIDRA v3** — API moderna que cobre todas as séries publicadas

Usamos exclusivamente o SIDRA. Os dados do FTP são follow-up para séries
que eventualmente não estejam no SIDRA.

## Dados agregados vs. desagregados

Esta primeira versão ingere **totais agregados**:
- PAM: total de lavouras (`c782/0`), não por cultura individual
- PPM: total de rebanhos (`c79/0`), não por espécie

A versão per-crop (PAM) e per-animal (PPM) gera dezenas de GB e é follow-up.

## Idempotência

**Raw (ingestão):** por recurso (`pesquisa + ano`). Re-ingerir o mesmo ano
sobrescreve o snapshot anterior (manifesto com ETag/SHA256).

**Silver (follow-up):** chave `pesquisa + código do município + ano`.

## Follow-up

- **Per-crop (PAM)** e **per-animal (PPM)** — desagregação por cultura/espécie
- **Decodificador Bronze** — JSON SIDRA → Arrow
- **Silver** — chave `pesquisa + município + ano`, join com outras fontes
- **Dados do FTP** — anos antigos ou tabelas não cobertas pelo SIDRA
- **Próximo Censo Agropecuário** — adicionar quando publicado
