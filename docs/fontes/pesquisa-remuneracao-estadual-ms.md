# Pesquisa de remuneração estadual de Mato Grosso do Sul

**Verificação:** 10/11/2026
**Escopo:** catálogo oficial Dados Abertos MS. Consultei apenas descrição e metadados dos recursos; não baixei nem abri CSVs ou registros nominais.

## Achados

- O conjunto [Folha de Pagamento](https://www.dados.ms.gov.br/dataset/folha-de-pagamento), publicado pelo Governo do Estado e fornecido pela Secretaria de Estado de Administração (SAD), descreve remuneração de servidores ativos e inativos, empregados públicos e pensionistas da administração direta e indireta. Os valores incluem remuneração base, outras verbas e remuneração após deduções obrigatórias; o catálogo explica quais classes de deduções não entram nos dados.
- O catálogo lista recursos CSV mensais de janeiro de 2018 a setembro de 2026, incluindo competências de 13º salário em vários anos. A página do recurso [Folha de Pagamento — 09/2026](https://www.dados.ms.gov.br/dataset/folha-de-pagamento/resource/40f995ce-64a6-415b-92fe-fe60d737e3bf) identifica formato CSV, tamanho de 14,2 MiB, licença Creative Commons Atribuição, URL do tipo `upload` e data de atualização de 16/10/2026. São metadados do catálogo; não comprovam a integridade do arquivo.
- O dataset declara licença Creative Commons Atribuição, sem indicar a versão na página consultada. Registrar atribuição à fonte e confirmar a versão aplicável antes de publicar derivados com condições de licenciamento específicas.
- O catálogo separa uma página por recurso e oferece controles “Mais informações” e “Baixar”. A página textual do recurso apresenta identificador e metadados, mas não expõe o endereço direto do arquivo. Não usei o controle de download nem inferi URLs a partir do funcionamento da interface. Ainda é necessário confirmar, pela interface oficial de API/metadados, como enumerar os recursos sem baixar o conteúdo.
- A legislação estadual tem uma fonte separada que fixa o subsídio do governador, vice-governador e secretários, com vigência a partir de 2023 ([Lei estadual nº 6.010/2022](https://aacpdappls.net.ms.gov.br/appls/legislacao/secoge/govato.nsf/2cab8d75940ca72e04256d1a004acf14/bee29e1bb55e39640425892000413843)). Isso documenta o subsídio legal, não o valor efetivamente pago em cada competência. A descrição do conjunto de folha não confirma se o governador está incluído.

## Avaliação

**Prontidão: melhor candidato documental para servidores; dados de governador precisam ser tratados separadamente.** O MS tem arquivos CSV mensais catalogados, competência explícita, tamanho e licença declarada; isso reduz as lacunas encontradas em fontes que só oferecem consultas ou interfaces dinâmicas. Há lei oficial sobre o subsídio do governador, mas falta fonte que confirme sua remuneração efetivamente paga em cada competência. Antes de implementar, confirmar a inclusão do governador na folha ou definir a lei/tabela vigente como fonte de subsídio legal; confirmar também que a API oficial do catálogo permite obter URLs, tamanho e competência sem solicitar os arquivos nem expor registros nominais.

Se aprovado, usar somente metadados para descobrir e selecionar recursos; preservar o arquivo original opaco na Raw, registrar competência e atribuição e não materializar linhas nominais em logs, testes ou documentação.

## Fontes oficiais

- [Dados Abertos MS — Folha de Pagamento](https://www.dados.ms.gov.br/dataset/folha-de-pagamento)
- [Dados Abertos MS — recurso Folha de Pagamento 09/2026](https://www.dados.ms.gov.br/dataset/folha-de-pagamento/resource/40f995ce-64a6-415b-92fe-fe60d737e3bf)
- [Legislação MS — Lei estadual nº 6.010/2022 sobre subsídio do governador](https://aacpdappls.net.ms.gov.br/appls/legislacao/secoge/govato.nsf/2cab8d75940ca72e04256d1a004acf14/bee29e1bb55e39640425892000413843)
