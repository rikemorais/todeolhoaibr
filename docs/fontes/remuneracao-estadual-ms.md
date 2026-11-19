# Remuneração Estadual — Mato Grosso do Sul

Fonte: **Dados Abertos MS** · Formato: CSV · Atualização: mensal ·
Licença: Creative Commons Atribuição

## O que é ingerido

| Recurso | Formato | Descrição |
|---------|---------|-----------|
| `ultima-competencia` | CSV | Folha de pagamento mensal mais recente listada no catálogo CKAN |

A fonte consulta apenas os metadados do catálogo (`package_show`) e preserva o
CSV opaco na Raw, sem interpretar registros nominais.

## Sobre a fonte

O [catálogo de Dados Abertos de MS](https://www.dados.ms.gov.br/dataset/folha-de-pagamento)
publica CSVs mensais de remuneração de servidores ativos, inativos, empregados
públicos e pensionistas da administração direta e indireta do Executivo estadual.
Recursos disponíveis desde janeiro de 2018, atualizados mensalmente.

O catálogo inclui meses "13" (décimo terceiro); a fonte os ignora e seleciona
apenas competências de 1 a 12.

## Perfil HTTP

- **Host com limite**: `www.dados.ms.gov.br` — 2 req/s
- **Autenticação**: nenhuma
- **Redirecionamento**: seguir

## Padrão de URL

Catálogo CKAN:
```
https://www.dados.ms.gov.br/api/3/action/package_show?id=folha-de-pagamento
```

Download (formato observado em novembro de 2026):
```
https://www.dados.ms.gov.br/dataset/{dataset-id}/resource/{resource-id}/download/folha-{MM}_{AAAA}.csv
```

## Comandos

```bash
docker compose run --rm --build ingestao executar remuneracao-estadual-ms --sim
```

## Observações

- **Sob demanda**: o CSV mensal tem ~15 MB.
- **Privacidade**: registros contêm dados pessoais (nome, matrícula, valores
  individualizados). A fonte não materializa nem inspeciona registros — preserva
  o arquivo opaco na Raw.
- **Mês 13**: o catálogo lista recursos com mês 13 (13° salário); a fonte
  filtra competências de 1 a 12 apenas.
