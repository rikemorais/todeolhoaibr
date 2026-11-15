# Fonte `cvm` — demonstrações financeiras de companhias abertas (DFP/ITR)

Demonstrações Financeiras Padronizadas (DFP, anual) e Informações Trimestrais (ITR) das empresas de **capital aberto**, publicadas pela CVM em [dados.cvm.gov.br](https://dados.cvm.gov.br). Implementa a issue [#37](https://github.com/rikemorais/todeolhoaibr/issues/37).

## O que é ingerido

- **DFP** — um ZIP por ano (2010–2026), ~8–13 MB cada. Dentro de cada ZIP há ~19 CSVs cobrindo 8 tipos de demonstração (BPA, BPP, DRE, DFC_MD, DFC_MI, DMPL, DRA, DVA) nas versões consolidada e individual, além de metadados (parecer, composição de capital, documento).
- **ITR** — um ZIP por ano (2011–2026), ~23–31 MB cada. Mesma estrutura do DFP, mas com dados trimestrais.
- Total: 33 arquivos, ~350 MB.
- CSVs em Latin-1 com `;` como separador.
- Gravado **opaco** na camada Raw (ver [ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md)).

### Recursos

| Recurso | URL | O que é |
| --- | --- | --- |
| `dfp/ano={YYYY}` | `dfp_cia_aberta_{YYYY}.zip` | Demonstrações financeiras anuais |
| `itr/ano={YYYY}` | `itr_cia_aberta_{YYYY}.zip` | Informações trimestrais |

### Tipos de demonstração no ZIP

| Código | Demonstração |
| --- | --- |
| BPA | Balanço Patrimonial — Ativo |
| BPP | Balanço Patrimonial — Passivo e Patrimônio Líquido |
| DRE | Demonstração de Resultado |
| DFC_MD | Fluxo de Caixa — Método Direto |
| DFC_MI | Fluxo de Caixa — Método Indireto |
| DMPL | Mutações do Patrimônio Líquido |
| DRA | Resultado Abrangente |
| DVA | Valor Adicionado |

## Recorte: apenas companhias abertas

A CVM cobre **apenas empresas de capital aberto** (listadas em bolsa). Não inclui MEI, empresas privadas ou sem registro na CVM. Para o universo completo de CNPJs, ver a fonte `cnpj` (Receita Federal).

## Layout na Raw

```text
raw/cvm/dfp/ano=2024/_ultima_versao.json
raw/cvm/dfp/ano=2024/data_ingestao=AAAA-MM-DD/execucao=<id>/parte-00001.zip
raw/cvm/itr/ano=2024/_ultima_versao.json
raw/cvm/itr/ano=2024/data_ingestao=AAAA-MM-DD/execucao=<id>/parte-00001.zip
```

## Como rodar

```bash
todeolho executar cvm
```

!!! warning "Sob demanda"
    ~350 MB total: a fonte não entra em `--todas` por padrão. Execute explicitamente com `todeolho executar cvm`.

## Idempotência

Cada ZIP anual tem ETag e Content-Length estáveis. A pré-checagem compara a versão com a da última ingestão (ver [ADR 0012](../adr/0012-pre-checagem-idempotente.md)). O ano corrente aceita 404 (o arquivo pode ainda não ter sido publicado).

## Observações de parsing

- **Encoding**: Latin-1 (ISO-8859-1).
- **Separador**: `;` (ponto-e-vírgula).
- **Escala monetária**: `ESCALA_MOEDA` pode ser `"MIL"` (valores em milhares) ou `"UNIDADE"`.
- **Reapresentações**: usar o maior `VERSAO` por empresa/exercício para obter a versão mais recente.
- **Capital de giro**: calculado (ativo circulante − passivo circulante), não vem pronto.
