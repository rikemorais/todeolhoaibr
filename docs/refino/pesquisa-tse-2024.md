# Pesquisa de identidade de linha — contas eleitorais de candidatos em 2024

**Escopo:** distinguir duplicatas exatas de linhas com a mesma sequência no [recurso oficial de prestação de contas de candidatos de 2024](https://dadosabertos.tse.jus.br/dataset/prestacao-de-contas-eleitorais-2024/resource/c74840a4-f303-4771-af44-ecbf15bfc302). Consulta em 2026-11-05; nenhuma linha ou identificador pessoal foi exibido.

## Fonte e semântica oficial

O [ZIP publicado pelo TSE](https://cdn.tse.jus.br/estatistica/sead/odsele/prestacao_contas/prestacao_de_contas_eleitorais_candidatos_2024.zip), presente na Raw local, contém quatro PDFs específicos: `leiame_despesas-contratadas-candidatos.pdf`, `leiame_despesas-pagas-candidatos.pdf`, `leiame_receitas-candidatos.pdf` e `leiame_receitas-candidatos-doador-originario.pdf`. Eles descrevem `SQ_PRESTADOR_CONTAS` como sequência do prestador por eleição, `SQ_DESPESA`/`SQ_RECEITA` como sequências de identificação de registros e `SQ_PARCELAMENTO_DESPESA` como sequência do pagamento. Também definem `#NULO`/`-1` para informação ausente e `#NE`/`-3` para informação não registrada naquele ano. Os PDFs não estabelecem que essas combinações sejam chaves únicas dos CSVs publicados; isso precisa ser medido.

## Evidência local agregada

As consultas usaram as quatro partes `BRASIL` do manifesto bronze vigente de `tse/prestacao-contas/ano=2024`. A chave testada em cada conjunto inclui `CD_ELEICAO` e `SQ_PRESTADOR_CONTAS` além das colunas indicadas.

| Conjunto | Sequência testada | Grupos repetidos | Linhas excedentes | Maior grupo |
| --- | --- | ---: | ---: | ---: |
| Despesas contratadas | `SQ_DESPESA` | 383.230 | 1.326.624 | 547 |
| Receitas | `SQ_RECEITA` | 71.814 | 142.992 | 119 |
| Despesas pagas | (`SQ_DESPESA`, `SQ_PARCELAMENTO_DESPESA`) | 56 | 79 | 5 |
| Doador originário | `SQ_RECEITA` | 4.762 | 81.777 | 1.251 |

Há `SQ_DESPESA=-1` em 159.100 linhas de despesas contratadas e `SQ_RECEITA=-1` em 66.599 linhas de receitas. Na tabela de doador originário, `SQ_RECEITA` e `NR_CPF_CNPJ_DOADOR_ORIGINARIO` são ambos `-1` em 433.385 linhas. **Excluir essas sentinelas não elimina as colisões:** despesas contratadas ainda têm 4.293.210 linhas e 2.966.586 chaves; receitas têm 1.974.460 e 1.831.468. Em doador originário, incluir o documento eleva a cardinalidade de 452.567 para 529.130, ainda abaixo das 534.344 linhas. A data de prestação **não** eleva a cardinalidade.

Nas partes de AC, 1.176 grupos repetidos de despesas contratadas variam em `DS_DESPESA` (1.142 grupos) ou `VR_DESPESA_CONTRATADA` (1.144); 37 grupos de receitas variam em `DS_RECEITA` e `VR_RECEITA`, e quatro também em `DS_NATUREZA_RECURSO_ESTIMAVEL`. Em quatro grupos repetidos de doador originário variam documento, nome e valor; a data da receita varia em um deles. Nenhuma outra coluna variou nesses grupos de AC. Os grupos repetidos de despesas contratadas e receitas de AC são do tipo final.

O CSV oficial de AC foi lido em fluxo diretamente do ZIP, sem criar cópia nem mostrar linhas: despesas contratadas têm 23.511 linhas e 18.211 chaves; receitas têm 6.313 linhas e 6.255 chaves. Os números coincidem com o Bronze, portanto essas colisões já existem na publicação da origem. A comparação exata das linhas das 56 chaves repetidas de despesas pagas no consolidado encontrou 135 linhas, das quais 56 são distintas: as 79 excedentes são cópias exatas na versão local.

## Política para o contrato Silver de 2024

1. Ler somente a representação `BRASIL` do manifesto bronze vigente; comparar sua cobertura com as partes por UF antes da primeira materialização.
2. Para **despesas pagas**, normalizar tipos e sentinelas, remover apenas linhas inteiramente idênticas, contar as remoções e exigir unicidade de (`AA_ELEICAO`, `CD_ELEICAO`, `SQ_PRESTADOR_CONTAS`, `SQ_DESPESA`, `SQ_PARCELAMENTO_DESPESA`). Se uma mesma chave tiver atributos diferentes numa versão futura, rejeitar ou interromper a publicação conforme o limiar do contrato; nunca escolher uma das linhas silenciosamente.
3. Para **despesas contratadas** e **receitas**, preservar todas as variantes. O mesmo número sequencial identifica mais de uma linha publicada no CSV; descrição e valor não são identidade estável. O grão de publicação e a identidade de cada linha ainda exigem decisão e referência estável à posição na origem. O [decodificador ZIP/CSV atual](../../crates/refino/src/csv_zip.rs) não grava ordinal de linha no Parquet; manifesto, parte e ordinal precisam compor a linhagem antes de publicar essas variantes. Esse identificador técnico vale apenas dentro da versão bronze e não substitui chave de negócio entre versões. Não materializar uma tabela Silver canônica desses conjuntos por deduplicação da sequência.
4. Para **doador originário**, não usar documento como chave universal: ele falta junto com `SQ_RECEITA` em 433.385 linhas, e (`SQ_RECEITA`, documento) ainda colide. Preservar variantes e esclarecer o grão antes da materialização canônica.

Os números e a política acima pertencem ao layout de 2024. Contratos de outros anos exigem validação própria. A hipótese de que as variantes são linhas de detalhe é uma **inferência** das contagens e dos campos que mudam, não uma afirmação dos PDFs do TSE.
