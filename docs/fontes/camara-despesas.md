# Fonte `camara-despesas` — despesas da CEAP

Despesas da **CEAP** (Cota para o Exercício da Atividade Parlamentar) dos deputados federais: "quanto um deputado gastou, com quem e quando". Implementa a [issue #25](https://github.com/rikemorais/todeolhoaibr/issues/25).

## O que é ingerido

- Um **arquivo CSV (zipado) por ano**, nacional, publicado pela Câmara em
  `https://www.camara.leg.br/cotas/Ano-{ano}.csv.zip`, de **2008** até o ano atual.
- O arquivo é baixado como **arquivo grande** (em fluxo, sem carregar na memória) e gravado
  **opaco** na camada Raw — a ingestão não descompacta nem interpreta o conteúdo (ver
  [ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md)).
- Documentação dos campos do CSV (fornecedor, valor, data, tipo etc.) pela própria Câmara:
  <https://dadosabertos.camara.leg.br/howtouse/2023-12-26-dados-ceap.html>.

## Layout na Raw

Mesmo sendo uma fonte separada no catálogo, os arquivos ficam sob a fonte `camara`:

```text
raw/camara/despesas/ano=AAAA/_ultima_versao.json                                    # ponteiro do vigente
raw/camara/despesas/ano=AAAA/data_ingestao=AAAA-MM-DD/execucao=<id>/parte-00001.zip
raw/camara/despesas/ano=AAAA/data_ingestao=AAAA-MM-DD/execucao=<id>/_manifesto.json
```

O `_ultima_versao.json` é o **ponteiro** que aponta para o manifesto da execução **vigente** de cada ano: um leitor a jusante começa por ele para achar o snapshot atual, em vez de adivinhar pelo caminho de execução (ver [ADR 0012](../adr/0012-pre-checagem-idempotente.md)).

## Fonte separada, `sob_demanda`

Embora a [issue #25](https://github.com/rikemorais/todeolhoaibr/issues/25) falasse em "recurso da
fonte `camara`", a CEAP é uma **fonte própria `camara-despesas` com `sob_demanda = true`**, para
**não entrar no `executar --todas`** (são ~19 arquivos anuais em lote). Como `sob_demanda` é por
fonte, não por tarefa, separá-la é a única forma de manter o `camara` (API leve) fora desse peso.
O dado, ainda assim, mora em `raw/camara/despesas/`.

## Como rodar

```bash
# Todos os anos (a fonte é argumento posicional)
todeolho executar camara-despesas

# Só um ano (filtro por etiqueta)
todeolho executar camara-despesas --filtro ano=2024
```

## Idempotência

Reusa a pré-checagem por versão/tamanho ([ADR 0012](../adr/0012-pre-checagem-idempotente.md)): a
origem envia um `ETag` (conferido no arquivo real), que o cliente usa como **validador
autoritativo** — um arquivo revisado ganha ETag novo e é reingerido; um inalterado é
reaproveitado. Quando não há validador, a checagem cai para o **tamanho** (a origem também manda
`Last-Modified`, mas o cliente hoje só considera o ETag).

## Fora de escopo desta camada (fica no refino)

- **Filtro por UF:** o arquivo é **nacional por ano**; a UF é uma **coluna do CSV**. Como a Raw é
  opaca, não há arquivo por UF para filtrar na ingestão — o recorte por UF é do refino/consulta.
- **Deduplicação por chave composta:** a CEAP **não tem ID único** por despesa. A deduplicação por
  registro é trabalho do refino (camada `silver`), não da ingestão — ver
  [ADR 0015](../adr/0015-arquitetura-de-dados-em-medalhao.md). A idempotência **da ingestão** é por
  versão do arquivo (acima).
