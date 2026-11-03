# Fonte `tse-prestacao-contas` — prestação de contas de campanha

Prestação de contas eleitorais do **TSE**: receitas e despesas de **campanha** dos candidatos — "quem doou, para quem, e no que o candidato gastou". Implementa a [issue #28](https://github.com/rikemorais/todeolhoaibr/issues/28).

!!! note "Campanha ≠ mandato"
    Isto é dinheiro de **campanha eleitoral**, não a cota de exercício do mandato. Para o gasto de mandato, veja [`camara-despesas`](camara-despesas.md) (CEAP, deputados) e [`senado-despesas`](senado-despesas.md) (CEAPS, senadores).

## O que é ingerido

- Um **arquivo ZIP por eleição**, nacional, do repositório de dados abertos do TSE
  (`https://cdn.tse.jus.br/estatistica/sead/odsele/prestacao_contas/`), de **2002** à eleição
  corrente (de dois em dois anos). O arquivo da eleição corrente pode ainda não estar publicado no
  começo do ano (HTTP 404): nesse caso a tarefa termina sem gravar e será tentada de novo; uma
  eleição histórica ausente continua sendo erro.
- O ZIP é baixado como **arquivo grande** (em fluxo, sem carregar na memória) e gravado **opaco**
  na camada Raw — a ingestão não descompacta nem interpreta o conteúdo (ver
  [ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md)).

### Convenção de nome por era

O TSE mudou o nome do arquivo consolidado ao longo dos anos; a fonte resolve isso por eleição:

| Eleição | Arquivo |
| --- | --- |
| 2002–2010 | `prestacao_contas_{ano}.zip` |
| 2012, 2016 | `prestacao_contas_final_{ano}.zip` |
| 2014 | `prestacao_final_{ano}.zip` |
| 2018 em diante | `prestacao_de_contas_eleitorais_candidatos_{ano}.zip` |

De 2018 em diante, o arquivo é específico de **candidatos** (receitas e despesas de candidatos). Até 2016, o ZIP consolidado traz a prestação no formato antigo do SPCE (candidatos, partidos e comitês juntos); separar o recorte de candidatos é trabalho do refino.

## Layout na Raw

Mesmo sendo uma fonte separada no catálogo, os arquivos ficam sob a fonte `tse`:

```text
raw/tse/prestacao-contas/ano=AAAA/_ultima_versao.json                                    # ponteiro do vigente
raw/tse/prestacao-contas/ano=AAAA/data_ingestao=AAAA-MM-DD/execucao=<id>/parte-00001.zip
raw/tse/prestacao-contas/ano=AAAA/data_ingestao=AAAA-MM-DD/execucao=<id>/_manifesto.json
```

## Fonte separada, `sob_demanda`

É uma **fonte própria `tse-prestacao-contas` com `sob_demanda = true`**, separada da
[`tse`](tse.md) (votação por seção). Assim cada uma roda sozinha — não faria sentido `executar tse`
baixar os ~16 GB de votação **e** a prestação de contas juntos. O dado, ainda assim, mora em
`raw/tse/prestacao-contas/`.

## Como rodar

```bash
# Todas as eleições (a fonte é argumento posicional)
todeolho executar tse-prestacao-contas

# Só uma eleição (filtro por etiqueta)
todeolho executar tse-prestacao-contas --filtro ano=2022
```

## Idempotência

Reusa a pré-checagem por versão/tamanho ([ADR 0012](../adr/0012-pre-checagem-idempotente.md)): a
origem envia um `ETag` (conferido no arquivo real), usado como **validador autoritativo** — um
arquivo revisado ganha ETag novo e é reingerido; um inalterado é reaproveitado.

## Fora de escopo desta camada (fica no refino)

- **Filtro por UF e separação receitas/despesas:** o ZIP é **nacional por eleição** e traz vários
  CSVs (receitas e despesas de candidatos, fornecedores, doadores). Como a Raw é opaca, o recorte
  por UF e a separação por tipo são do refino/consulta, não da ingestão.
- **Deduplicação por chave:** a chave adequada de cada registro (despesa/receita) depende do layout
  do CSV e é tratada no refino (camada `silver`) — ver
  [ADR 0015](../adr/0015-arquitetura-de-dados-em-medalhao.md). A idempotência **da ingestão** é por
  versão do arquivo (acima).
