# IBGE — Pesquisas Agropecuárias (SIDRA)

Fonte: **apisidra.ibge.gov.br** · Formato: JSON · Atualização: anual (PAM,
PPM, PEVS) / mensal (LSPA) · Licença: dados abertos

## O que é ingerido

Quatro pesquisas do sistema SIDRA do IBGE:

| Pesquisa | Tabela | Nível | Período | Variáveis |
|----------|--------|-------|---------|-----------|
| **PAM** (Produção Agrícola Municipal) | 5457 | N6 (municipal) | 1974–2025 | área plantada/colhida, quantidade, valor e rendimento (total agregado) |
| **PPM — rebanhos** (Pecuária Municipal) | 3939 | N6 (municipal) | 1974–2025 | efetivo dos rebanhos (total agregado) |
| **PPM — leite** (Pecuária Municipal) | 74 | N6 (municipal) | 1974–2025 | produção, valor |
| **PEVS** (Extração Vegetal e Silvicultura) | 289 | N6 (municipal) | 1986–2025 | quantidade e valor da produção extrativa |
| **LSPA** (Levantamento Sist. da Prod. Agrícola) | 6588 | N3 (estadual) | 2006–2026 | área plantada/colhida, produção, rendimento (mensal) |

**Total: 217 tarefas (~1,2 GB), sob demanda.**

### LSPA: nível estadual

A tabela 6588 do SIDRA **não suporta** o nível municipal (`N6`).
A API retorna `"Parâmetro N6 incompatível com a tabela"`.
A LSPA é ingerida no nível estadual (`N3`, 27 UFs).

### Censo Agropecuário: removido

A tabela 6881 causa `OutOfMemoryException` no SIDRA para `N6/all` e retorna
vazia para variáveis específicas. Requer estratégia de divisão por UF ou FTP;
fica como follow-up.

## API SIDRA

Padrão de URL (municipal):
```
https://apisidra.ibge.gov.br/values/t/{tabela}/n6/all/v/all/p/{periodo}[/{classificação}]
```

Padrão de URL (estadual, LSPA):
```
https://apisidra.ibge.gov.br/values/t/{tabela}/n3/all/v/all/p/{periodo}
```

- Retorna JSON (array de objetos), sem autenticação
- `n6/all` = todos os municípios (~5.570)
- `n3/all` = todos os estados (27)
- `v/all` = todas as variáveis da tabela
- Para PAM: `/c782/0` (classificação "Produto das lavouras", total)
- Para PPM-rebanhos: `/c79/0` (classificação "Tipo de rebanho", total)
- Para LSPA: períodos mensais (`{ano}01,{ano}02,...,{ano}12`)
- Redirects bloqueados (a API não usa redirects legítimos)

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
- PPM-rebanhos: total de rebanhos (`c79/0`), não por espécie

A versão per-crop (PAM) e per-animal (PPM) gera dezenas de GB e é follow-up.

## PPM: séries parciais

A issue pede leite, ovos e mel. As tabelas de ovos (75) e mel (76)
**não suportam** nível municipal (`N6`), apenas estadual (`N3`).
A tabela de leite (74) suporta `N6` e está incluída nesta versão.
Ovos e mel estaduais ficam como follow-up.

## Idempotência

**Raw (ingestão):** por recurso (`pesquisa + ano`). Re-ingerir o mesmo ano
sobrescreve o snapshot anterior (manifesto com ETag/SHA256).

**Silver (follow-up):** chave `pesquisa + código do município + ano`.

## Follow-up

- **Per-crop (PAM)** e **per-animal (PPM)** — desagregação por cultura/espécie
- **PPM ovos e mel** — tabelas 75 e 76 no nível estadual (`N3`)
- **Censo Agropecuário** — tabela 6881 com divisão por UF ou via FTP
- **Decodificador Bronze** — JSON SIDRA → Arrow
- **Silver** — chave `pesquisa + município + ano`, join com outras fontes
- **Dados do FTP** — anos antigos ou tabelas não cobertas pelo SIDRA
