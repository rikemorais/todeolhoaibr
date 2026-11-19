# Macro Internacional Comparado

Fonte: **IMF + OECD** · Formato: XLSX (IMF) + CSV/SDMX (OECD) ·
Atualização: semestral (WEO) / contínua (OECD) · Dados públicos

## O que é ingerido

### IMF (2 recursos)

| Recurso | Formato | Descrição |
|---------|---------|-----------|
| `weo-oct-2026` | XLSX | World Economic Outlook — edição Out/2026, dataset completo (países, grupos, commodities): PIB, inflação, desemprego, balanço fiscal, conta corrente, 1980–2031 |
| `weo-historical` | XLSX | Arquivo histórico de projeções do WEO (todas as vintages) |

### OECD (5 recursos)

| Recurso | Dataflow SDMX | Descrição |
|---------|---------------|-----------|
| `economic-outlook` | `DSD_EO@DF_EO` | Economic Outlook 119 — projeções macro anuais (PIB, inflação, desemprego, fiscal, comércio) por país |
| `gdp-expenditure` | `DSD_NAMAIN10@DF_TABLE1_EXPENDITURE` | PIB anual e componentes (despesa): consumo, investimento, exportações, importações |
| `cpi` | `DSD_PRICES@DF_PRICES_ALL` | Índices de preços ao consumidor (CPI/HICP), COICOP 1999, mensal e anual |
| `unemployment` | `DSD_LFS@DF_IALFS_UNE_M` | Taxa de desemprego mensal por sexo, faixa etária e atividade |
| `balance-of-payments` | `DSD_BOP@DF_BOP` | Balanço de pagamentos — conta corrente, capital, financeira |

## Sobre as fontes

- **[IMF](https://data.imf.org/)**: Fundo Monetário Internacional. O WEO é
  publicado em abril e outubro com projeções macro para ~190 países.
- **[OECD](https://sdmx.oecd.org/)**: Organização para Cooperação e
  Desenvolvimento Econômico. Dados via API SDMX REST em CSV, cobrindo
  países membros e parceiros (inclusive Brasil).

## Perfil HTTP

- **Host com limite**: `sdmx.oecd.org` — 1 req/2s (conservador)
- **Autenticação**: nenhuma
- **Redirecionamento**: seguir

## Padrão de URL

IMF (bulk download):
```
https://data.imf.org/-/media/iData/External-Storage/Documents/{ID}/en/{filename}.xlsx
```

OECD (SDMX REST):
```
https://sdmx.oecd.org/public/rest/data/{AGENCY},{DATAFLOW},/{KEY}?format=csvfilewithlabels
```

## Comandos

```bash
docker compose run --rm --build ingestao executar macro_intl --sim
```

Só IMF:
```bash
docker compose run --rm --build ingestao executar macro_intl --filtro orgao=imf
```

Só OECD:
```bash
docker compose run --rm --build ingestao executar macro_intl --filtro orgao=oecd
```

## Observações

- **Sob demanda**: os datasets OECD são pesados (EO ~103 MB, desemprego
  ~165 MB, CPI ~74 MB).
- **Cobertura**: IMF cobre ~190 países; OECD cobre membros + parceiros
  (Brasil incluído).
- **Natureza comparativa**: dados por país (não município IBGE) e em
  inglês — camada de benchmark internacional.
