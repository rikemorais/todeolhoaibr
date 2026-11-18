# SEEG — Emissões de Gases de Efeito Estufa

Fonte: **seeg.obass.haus** · Formato: XLSX · Atualização: anual · Licença:
dados abertos (Observatório do Clima)

## O que é ingerido

Dois arquivos da edição **SEEG 14.0** (novembro de 2026):

| Arquivo | URL | ~MB |
|---------|-----|----:|
| Dados nacionais (BR + UF) | `seeg.obass.haus/.../Dados-nacionais-14.0.xlsx` | 207 |
| Dados municipais resumido (CO2e GWP-AR5) | `seeg.obass.haus/.../Dados-municipais-resumido-CO2e-GWP-AR5-14.0.xlsx` | 229 |

**Total: 2 tarefas (~436 MB), sob demanda.**

## Sobre o SEEG

O Sistema de Estimativas de Emissões e Remoções de Gases de Efeito Estufa
(SEEG) é uma iniciativa do Observatório do Clima que estima as emissões
brasileiras de GEE desde 1990 até o ano mais recente disponível.

Cobertura por cinco setores:

- **Agropecuária** — fermentação entérica, manejo de dejetos, solos agrícolas
- **Energia** — transporte, geração elétrica, indústria, residencial
- **Mudança de Uso da Terra e Florestas** — desmatamento, regeneração
- **Processos Industriais e Uso de Produtos** — cimento, química, metais
- **Resíduos** — aterros, efluentes, incineração

Dados expressos em CO2 equivalente (toneladas), usando potencial de
aquecimento global GWP-AR5 (IPCC Fifth Assessment Report).

## Origem dos dados

A plataforma principal (`plataforma.seeg.eco.br`) retorna 403 via curl
(proteção CloudFront). Os downloads são hospedados em `seeg.obass.haus`
com URLs estáveis e HTTP direto.

Há também uma pasta no Google Drive com CSVs por UF, mas as URLs mudam
a cada edição e requerem autenticação para download programático.

## Idempotência

**Raw (ingestão):** por recurso (`dados-nacionais`, `dados-municipais`).
Re-ingerir o mesmo arquivo sobrescreve o snapshot anterior
(manifesto com ETag/SHA256).

**Silver (follow-up):** chave `setor + município + ano`.

## Follow-up

- **Decodificador Bronze** — XLSX → Arrow (5 setores × recortes territoriais)
- **Silver** — chave `setor + município + ano`, join com outras fontes
- **CSVs do Google Drive** — se a URL se estabilizar ou for feito scraping
- **Novas edições** — ao sair SEEG 15.0, atualizar as URLs (versionadas no nome)
