# Pesquisa de leiaute — receitas partidárias eleitorais de 2012

**Escopo:** identificar, em fontes oficiais do TSE, o significado das 19 posições dos arquivos `receitas_partidos_2012_<sigla>.txt` para definir um contrato e uma chave de negócio na Silver. Consulta realizada em 2026-11-05. Nenhum mapeamento foi presumido a partir do conteúdo de registros.

## Fontes oficiais e achados

1. O [conjunto Prestação de Contas Eleitorais - 2012](https://dadosabertos.tse.jus.br/dataset/prestacao-de-contas-eleitorais-2012) identifica o recurso [Prestação de contas final](https://dadosabertos.tse.jus.br/dataset/prestacao-de-contas-eleitorais-2012/resource/056e7001-622a-4b09-8d1c-cd3a1936925d) como ZIP/TXT oriundo do sistema SPCE. O [ZIP publicado pelo TSE](https://cdn.tse.jus.br/estatistica/sead/odsele/prestacao_contas/prestacao_final_2012.zip) é a origem da Raw local.
2. O [grupo oficial de prestação de contas eleitorais](https://dadosabertos.tse.jus.br/group/about/prestacao-de-contas-eleitorais) alerta que cada eleição pode ter estrutura própria e orienta consultar o `leia-me.pdf` contido no ZIP.
3. No ZIP local correspondente ao recurso oficial, há somente um documento: `LEIAME.pdf`. Seu texto lista leiautes de eleitorado, candidaturas, bens e votação; não contém entrada para prestação de contas, receitas ou despesas. Portanto ele não documenta as posições dos arquivos `receitas_partidos_2012_<sigla>.txt`.
4. O mesmo ZIP contém 27 arquivos `receitas_partidos_2012_<sigla>.txt` com sufixo de duas letras, incluindo `BR`, e um arquivo `receitas_partidos_2012_brasil.txt`. O primeiro registro do arquivo `MT` tem 19 campos separados por `;` e não é cabeçalho. Já `..._brasil.txt` tem cabeçalho com **25** campos. Os dois formatos não são intercambiáveis para nomear posições.

## Resultado para a Silver

Não foi localizado um dicionário oficial específico para as 19 posições. A chave de negócio e a tipagem semântica dessa família devem permanecer pendentes; o contrato não deve atribuir os 25 nomes do arquivo consolidado às 19 posições. A próxima fonte primária a solicitar é o leiaute de exportação do SPCE 2012 à área gestora TSE/ASEPA, indicada no [catálogo oficial](https://dadosabertos.tse.jus.br/dataset/prestacao-de-contas-eleitorais-2012).

## Como a verificação local foi feita

O ZIP observado é `raw/tse/prestacao-contas/ano=2012/data_ingestao=2026-11-03/execucao=20261103T112315Z-69048fae/parte-00001.zip`. Foram inspecionados nomes de membros do ZIP, as notações de tabela no texto extraído do PDF e apenas a quantidade de campos da primeira linha dos dois TXT. Nenhum registro individual nem identificador pessoal foi copiado para esta nota.
