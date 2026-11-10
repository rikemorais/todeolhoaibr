# Pesquisa de remuneração estadual da Paraíba

**Verificação:** 10/11/2026

**Escopo:** documentação oficial da SEAD, CODATA, Portal da Transparência e portal estadual de dados abertos. Não chamei a API de folha, não baixei arquivos e não acessei registros nominais.

## Achados

- O catálogo oficial da SEAD publica o conjunto [Remuneração Servidores](https://dados.pb.gov.br/dataset/remuneracao-servidores), com recursos JSON separados para remuneração de servidores, diárias, órgãos da folha e estagiários. A descrição informa que cobre dados da folha estadual e o catálogo declara licença Creative Commons Atribuição e Compartilhamento pela mesma Licença (CC BY-SA). A fonte e o mantenedor são a SEAD-PB.
- A [especificação OpenAPI oficial da CODATA](https://api.dadosabertos.codata.pb.gov.br/api/v1/swagger.json) documenta `GET /api/v1/remuneracao/servidor`. Exige `ano` e `mes`; filtros opcionais incluem CNPJ do órgão, cargo, situação, regime contratual e nome. A paginação usa `page` (padrão 1) e `per_page` (padrão 100, máximo 1.000). A especificação descreve resposta paginada e respostas 400/404. Consultei o documento OpenAPI, sem chamar a operação de folha.
- O [CKAN oficial da SEAD-PB](https://dados.pb.gov.br/api/3/action/package_show?id=remuneracao-servidores), consultado por metadados em 10/11/2026, lista quatro recursos JSON; o recurso de remuneração não tem `url`, tamanho ou `last_modified`, e o conjunto declara `cc-by-sa`. A descrição do próprio recurso fornece o endpoint da API e um exemplo de consulta. O catálogo foi criado em janeiro de 2026 e modificado em 04/08/2026; essa data é de metadados, não comprova competência da folha.
- O [Portal da Transparência da Paraíba](https://transparencia.pb.gov.br/remuneracao-servidores) exibia agosto/2026 como competência selecionada e informava última atualização em 20/09/2026, com fonte SEAD. A página é JavaScript e não documenta um mecanismo de metadados para descobrir automaticamente a competência mais recente sem consultar a API que retorna os registros.
- O painel apresenta filtros de ano, mês, órgão, situação, regime, cargo e nome; também mostra categorias de vínculo como ativos, aposentados e pensionistas. Isso confirma a amplitude dos filtros e das categorias agregadas, mas não comprova sozinho que o recurso de API individual contenha todos esses vínculos ou inclua o governador.
- A licença apresenta divergência entre páginas oficiais: o conjunto específico declara CC BY-SA, enquanto o rodapé do [Portal Dados Abertos](https://dados.pb.gov.br/) declara CC BY 4.0 para os dados do portal. Para a folha nominal, aplicar a licença específica do conjunto e registrar a atribuição; confirmar com a SEAD/CODATA qual licença prevalece antes de republicar derivados sob condições diferentes.
- O Decreto estadual nº 46.927/2025 institui a Política de Dados Abertos e designa `dados.pb.gov.br` como plataforma oficial, com API, download e metadados. O decreto também exige observar proteção de dados pessoais e mencionar a fonte. Ele dá contexto institucional, mas não fornece o esquema ou os limites do recurso de remuneração.

## Avaliação

**Prontidão: fonte promissora, ainda sem competência automatizável confirmada.** O contrato da API, a paginação e o limite por chamada estão documentados; a API é paginada e não fornece um recurso estático para preservar diretamente como um único arquivo. O catálogo também não publica URL nem tamanho do recurso JSON. Falta descobrir a competência mais recente sem consultar respostas com registros e confirmar se a folha inclui o governador.

A licença CC BY-SA específica do conjunto é favorável à reutilização sob atribuição e compartilhamento pela mesma licença, mas a diferença para o aviso global CC BY 4.0 precisa ser esclarecida para decidir sobre derivados. Se a fonte for implementada, manter os dados nominais opacos na Raw e não os materializar em documentação, logs ou testes com dados reais.

## Fontes oficiais

- [SEAD-PB — conjunto Remuneração Servidores](https://dados.pb.gov.br/dataset/remuneracao-servidores)
- [CODATA-PB — Swagger da API de Dados Abertos](https://api.dadosabertos.codata.pb.gov.br/swagger/)
- [CODATA-PB — especificação OpenAPI JSON](https://api.dadosabertos.codata.pb.gov.br/api/v1/swagger.json)
- [SEAD-PB — API CKAN de metadados do conjunto](https://dados.pb.gov.br/api/3/action/package_show?id=remuneracao-servidores)
- [Portal da Transparência PB — Remuneração de Servidores](https://transparencia.pb.gov.br/remuneracao-servidores)
- [Portal de Dados Abertos PB](https://dados.pb.gov.br/)
- [Decreto estadual nº 46.927, de 7 de agosto de 2025](https://transparencia.pb.gov.br/pdfs/decreto-ndeg-46-927-de-07-de-agosto-de-2025.pdf/%40%40download/file/DECRETO%20N%C2%B0%2046.927%20DE%2007%20DE%20AGOSTO%20DE%202025.pdf)
