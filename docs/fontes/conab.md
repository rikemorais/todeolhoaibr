# CONAB — Séries de Safra de Grãos

Fonte: **conab** (`FonteConab`)

A CONAB (Companhia Nacional de Abastecimento) publica boletins mensais com
estimativas de safra de grãos. Cada boletim é um XLSX com ~76 abas: resumo
nacional por produto, dados por UF, e detalhamento por cultura.

## Levantamentos

Cada safra (ex: 2025/26) tem até 12 levantamentos publicados mensalmente de
outubro a setembro:

| Lev. | Mês | Lev. | Mês |
|------|-----|------|-----|
| 1º | Outubro | 7º | Abril |
| 2º | Novembro | 8º | Maio |
| 3º | Dezembro | 9º | Junho |
| 4º | Janeiro | 10º | Julho |
| 5º | Fevereiro | 11º | Agosto |
| 6º | Março | 12º | Setembro |

## Recursos

Padrão: `safras/safra={YYYY-YY}/levantamento={NN}`

Exemplo: `safras/safra=2025-26/levantamento=12` (12º levantamento da safra 2025/26).

Safras cobertas: 2024/25, 2025/26, 2026/27 (levantamentos futuros aceitam ausência).

## Formato

XLSX (~1 MB). A aba "Brasil - Total por Produto" contém uma linha por produto
com métricas de duas safras (anterior e atual).

## Bronze

Decodificador: `DecodificadorConabSafra` — extrai a aba de resumo nacional e
emite linhas flat com 5 colunas:

| Coluna | Descrição |
|--------|-----------|
| `produto` | Nome do produto (SOJA, MILHO TOTAL, ARROZ, etc.) |
| `safra` | Ano-safra (ex: 24/25) |
| `area_mil_ha` | Área plantada em mil hectares |
| `produtividade_kg_ha` | Produtividade em kg/ha |
| `producao_mil_t` | Produção em mil toneladas |

## Observações

- O XLSX é baixado pelo link de "Tabela de dados" publicado na página oficial do boletim. Os nomes variam entre levantamentos: parte de 2024/25 usa o slug `tabela-de-dados-producao-e-balanco-de-oferta-e-demanda-de-graos`, e alguns links não terminam em `.xlsx`. Se o portal recusar a consulta `HEAD` com HTTP 403, o cliente segue sem ETag/tamanho e tenta o `GET`; um 403 no próprio download continua sendo tratado como erro de acesso.
- O portal antigo (`portaldeinformacoes.conab.gov.br`) está offline. Os boletins
  de safra estão disponíveis em `gov.br`.
- Estoques e preços agrícolas não estão incluídos nesta fonte (portal offline).
- Cada produto gera 2 linhas Bronze (uma por safra comparada no boletim).

## Referências

- [CONAB — Boletim da Safra de Grãos](https://www.gov.br/conab/pt-br/atuacao/informacoes-agropecuarias/safras/safra-de-graos/boletim-da-safra-de-graos)
- [CONAB — Informações Agropecuárias](https://www.gov.br/conab/pt-br/atuacao/informacoes-agropecuarias)
