# Servidores federais ativos — Portal da Transparência

A fonte `transparencia-servidores-ativos` baixa os arquivos mensais de remuneração
dos servidores civis ativos do Executivo Federal publicados pelo Portal da
Transparência. Ela preserva os ZIPs opacos na Raw, por origem e competência; não
extrai nem expõe dados pessoais em tabelas analíticas ou no painel.

## Origem e escopo

O [Portal de downloads](https://portaldatransparencia.gov.br/download-de-dados/servidores)
publica arquivos mensais e oferece as planilhas SIAPE e BACEN. A API de remuneração
é individual por servidor e competência, portanto não é adequada para esta coleta
nacional em lote. Os arquivos podem conter nome, CPF e componentes remuneratórios;
o acesso ao bucket Raw deve seguir a governança de dados pessoais do projeto.

| Origem | Arquivo oficial | Recurso Raw |
| --- | --- | --- |
| SIAPE | `AAAAMM_Servidores_SIAPE.zip` | `servidores-ativos/origem=siape/competencia=AAAA-MM/` |
| BACEN | `AAAAMM_Servidores_BACEN.zip` | `servidores-ativos/origem=bacen/competencia=AAAA-MM/` |

O intervalo catalogado começa em 2020-01 e termina dois meses antes da execução,
para evitar competências que ainda podem estar sendo publicadas. Cada origem e
competência possui manifesto próprio e pode ser reexecutada sem baixar novamente
um arquivo inalterado. Os arquivos são obtidos pela página oficial de download,
que redireciona ao host de dados abertos.

## Como executar

A fonte é sob demanda e não entra em `executar --todas`. Filtre por uma origem e
uma competência, pois cada ZIP pode ser grande:

```bash
docker compose run --rm --build ingestao executar transparencia-servidores-ativos \
  --filtro origem=siape \
  --filtro competencia=2026-01
```

Valores aceitos para `origem`: `siape` e `bacen`. `competencia` usa `AAAA-MM`.
O download em lote não requer `PORTAL_TRANSPARENCIA_TOKEN`.

## Fontes oficiais

- [Download de servidores do Portal da Transparência](https://portaldatransparencia.gov.br/download-de-dados/servidores)
- [Dicionário geral de dados de servidores](https://portaldatransparencia.gov.br/dicionario-de-dados/servidores-geral)
- [Dicionário de cadastro de servidores](https://portaldatransparencia.gov.br/dicionario-de-dados/servidores-cadastro)
- [Dicionário de remuneração de servidores](https://portaldatransparencia.gov.br/dicionario-de-dados/servidores-remuneracao)
- [Swagger da API do Portal da Transparência](https://api.portaldatransparencia.gov.br/swagger-ui/index.html)
