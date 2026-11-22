# Bolsas Globais — FRED

Fonte: **Federal Reserve Economic Data (FRED)** · Formato: CSV · Atualização:
diária (dias úteis) · Licença: dados públicos, uso livre com atribuição

## O que é ingerido

### Índices de bolsa

| Recurso | Código FRED | Descrição |
|---------|-------------|-----------|
| `sp500` | SP500 | S&P 500 (EUA) |
| `djia` | DJIA | Dow Jones Industrial Average (EUA) |
| `nasdaq` | NASDAQCOM | NASDAQ Composite (EUA) |
| `nikkei225` | NIKKEI225 | Nikkei 225 (Japão) |

### Câmbio

| Recurso | Código FRED | Descrição |
|---------|-------------|-----------|
| `usd-eur` | DEXUSEU | Dólar / Euro |
| `jpy-usd` | DEXJPUS | Iene / Dólar |
| `brl-usd` | DEXBZUS | Real / Dólar |
| `cny-usd` | DEXCHUS | Yuan / Dólar |
| `cad-usd` | DEXCAUS | Dólar canadense / Dólar |
| `chf-usd` | DEXSZUS | Franco suíço / Dólar |
| `mxn-usd` | DEXMXUS | Peso mexicano / Dólar |
| `usd-index` | DTWEXBGS | Índice ponderado do dólar (Broad) |

### Volatilidade e commodities

| Recurso | Código FRED | Descrição |
|---------|-------------|-----------|
| `vix` | VIXCLS | CBOE Volatility Index (VIX) |
| `petroleo-wti` | DCOILWTICO | Petróleo WTI (USD/barril) |
| `petroleo-brent` | DCOILBRENTEU | Petróleo Brent (USD/barril) |

Cada CSV contém toda a série histórica (10+ anos para a maioria), preservada
opaca na Raw.

## Sobre a fonte

O [FRED](https://fred.stlouisfed.org/) do Federal Reserve Bank of St. Louis
disponibiliza mais de 816.000 séries econômicas. Os CSVs são acessíveis sem
autenticação, com atualização diária nos dias úteis.

## Perfil HTTP

- **Host com limite**: `fred.stlouisfed.org` — 1 req/2s
- **Autenticação**: nenhuma
- **Redirecionamento**: seguir

## Padrão de URL

```
https://fred.stlouisfed.org/graph/fredgraph.csv?id={CODIGO_FRED}
```

## Comandos

```bash
# Todas as séries
docker compose run --rm --build ingestao executar bolsas-globais --sim

# Apenas índices de bolsa
docker compose run --rm --build ingestao executar bolsas-globais --filtro categoria=indice --sim

# Apenas câmbio
docker compose run --rm --build ingestao executar bolsas-globais --filtro categoria=cambio --sim

# Apenas S&P 500
docker compose run --rm --build ingestao executar bolsas-globais --filtro serie=sp500 --sim
```

## Observações

- **Sob demanda**: os CSVs são pequenos (< 1 MB cada). Volume total < 10 MB.
- **Sem API de metadados**: as URLs são estáticas; o código FRED de cada série é fixo.
- **Identificação HTTP**: a borda do FRED deixa requisições com o agente padrão sem resposta; esta fonte usa um agente compatível com curl que também identifica o projeto.
- **Dias sem valor**: o FRED preenche com "." nos feriados/fins de semana.
- **Alternativas descartadas**: Stooq (proteção anti-bot com PoW/JS), Yahoo Finance (requer login), Alpha Vantage (requer chave API).
