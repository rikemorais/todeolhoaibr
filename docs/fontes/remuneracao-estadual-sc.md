# Remuneração Estadual — Santa Catarina

Fonte: **Dados Abertos SC** · Formato: CSV · Atualização: mensal ·
Licença: Creative Commons Atribuição

## O que é ingerido

| Recurso | Formato | Descrição |
|---------|---------|-----------|
| `ativos-ultima-competencia` | CSV | Servidores ativos — competência mais recente listada no catálogo |
| `inativos-ultima-competencia` | CSV | Servidores inativos — competência mais recente listada no catálogo |

A fonte consulta apenas os metadados do catálogo (`package_show`) e preserva
os CSVs opacos na Raw, sem interpretar registros nominais.

## Sobre a fonte

O [catálogo de Dados Abertos de SC](https://dados.sc.gov.br/dataset/remuneracao-servidores)
publica CSVs mensais separados para servidores ativos e inativos do Executivo
estadual. Dados disponíveis desde abril de 2022, atualizados mensalmente.

## Perfil HTTP

- **Host com limite**: `dados.sc.gov.br` — 2 req/s
- **Autenticação**: nenhuma
- **Redirecionamento**: seguir

## Padrão de URL

Catálogo CKAN:
```
https://dados.sc.gov.br/api/3/action/package_show?id=remuneracao-servidores
```

Download (formato observado em novembro de 2026):
```
https://dados.sc.gov.br/dataset/{dataset-id}/resource/{resource-id}/download/servidores-ativos-{AAAA}-{MM}.csv
https://dados.sc.gov.br/dataset/{dataset-id}/resource/{resource-id}/download/servidores-inativos-{AAAA}-{MM}.csv
```

## Comandos

```bash
docker compose run --rm --build ingestao executar remuneracao-estadual-sc --sim
```

## Observações

- **Sob demanda**: os CSVs mensais têm ~22 MB (ativos) e ~14 MB (inativos).
- **Privacidade**: registros contêm dados pessoais. A fonte não materializa
  nem inspeciona registros — preserva os arquivos opacos na Raw.
- **Dois arquivos por competência**: ativos e inativos são publicados separadamente.
