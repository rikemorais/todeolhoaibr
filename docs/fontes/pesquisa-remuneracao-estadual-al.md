# Pesquisa de remuneração estadual de Alagoas

**Verificação:** 10/11/2026
**Escopo:** documentação e páginas oficiais do Portal da Transparência de Alagoas, mantido pela CGE-AL. Não consultei os endpoints de folha, não baixei arquivos TXT e não acessei registros nominais.

## Achados

- A documentação oficial descreve uma API JSON para consultas do portal e informa que a página de Download de Dados oferece arquivos TXT com tabelas do banco. A própria página do download diz que os arquivos podem ser reutilizados sob a licença Creative Commons BY-SA adotada pelo portal. [API Transparência](https://www.transparencia.al.gov.br/portal/api) · [Download de Dados](https://www.transparencia.al.gov.br/portal/download-de-dados)
- A documentação da [lista de servidores ativos](https://www.transparencia.al.gov.br/portal/api/pessoal/servidores-ativos/lista-de-servidores) descreve o endpoint `GET /pessoal/json-servidores/`. A consulta exige `ano`, `mes`, `limit` e `offset`; nome e código do órgão são filtros opcionais. A página orienta chamadas AJAX com o cabeçalho `X-Requested-With: XMLHttpRequest`.
- A [lista de servidores inativos](https://www.transparencia.al.gov.br/portal/api/pessoal/servidores-inativos/lista-de-inativos) também exige `ano`, `mes`, `limit` e `offset`; vínculo é opcional e pode filtrar aposentados ou pensionistas. A documentação confirma, portanto, endpoints paginados separados para ativos e inativos.
- A página oficial de [exportação da API](https://www.transparencia.al.gov.br/portal/api/exportacao) documenta geração de CSV, XLS e PDF por meio do serviço de relatórios. Para essa exportação, o parâmetro `limit` é descartado e o serviço retorna todos os resultados. Isso pode produzir arquivos grandes; a página não publica limite de tamanho, tempo máximo ou mecanismo de retomada.
- A página de [Download de Dados](https://www.transparencia.al.gov.br/portal/download-de-dados) lista tabelas “Folha Ativo” e “Folha Inativo”, separadas conforme as tabelas SQL do portal. O conteúdo confirma uma alternativa em lote, mas a página consultada não documenta para essas tabelas uma URL estável, tamanho, data de atualização, formato interno além de TXT ou esquema.
- A página inicial do portal informa que algumas consultas, incluindo folha de servidores, usam ano e mês. Porém, as páginas da API consultadas não indicam uma rota de metadados ou calendário que revele a competência mais recente sem consultar a folha. Não é possível afirmar, apenas pela documentação, qual competência está hoje completa nem como descobri-la sem chamar um endpoint que retorna registros.
- O portal declara conteúdo e dados sob licença Creative Commons; a página de download especifica CC BY-SA. A documentação não identifica a versão da licença nem esclarece se transformações específicas mantêm essa mesma condição. Registrar atribuição e compartilhamento pela mesma licença na procedência e confirmar com a CGE-AL antes de publicar derivados sob condições diferentes.
- A documentação do portal descreve apoio dos Poderes Legislativo e Judiciário, mas isso não comprova que os conjuntos de folha ativa/inativa da API cubram todos os poderes, órgãos, empresas ou vínculos. Também não confirma uma série específica do subsídio do governador.

## Avaliação

**Prontidão: tecnicamente promissora, mas requer definição de competência e estratégia de preservação antes da integração.** Diferente de uma consulta filtrada obrigatoriamente por nome, a API de ativos e inativos documenta paginação e não exige filtro nominal. A consulta, entretanto, requer mês e ano; não há descoberta de competência mais recente documentada. O portal também oferece arquivos completos em TXT, mas não documenta nesta página seu endereço, volume, atualização ou esquema.

Antes de implementar:

1. Confirmar com a CGE-AL como determinar a última competência publicada sem depender de tentativa sobre consultas nominais, ou definir competência explícita configurada pelo operador.
2. Confirmar URL, cabeçalhos, limite seguro de página e comportamento de paginação dos endpoints em documentação oficial. Não assumir que o exemplo/contrato da consulta avançada vale para as rotas simples de ativos e inativos.
3. Identificar a URL e metadados dos arquivos TXT de Folha Ativo e Folha Inativo, se essa alternativa for preferida; não baixar os arquivos para esta pesquisa.
4. Registrar licença CC BY-SA, escopo comprovado e lacunas de cobertura na procedência; não afirmar cobertura do subsídio do governador sem fonte específica.

Se a ingestão for aprovada, tratar o conteúdo como dado pessoal e manter os arquivos nominais opacos na Raw, sem materializar registros em documentação, logs ou smoke tests. A página de exportação ignora o `limit`, então preferir paginação JSON com limites controlados se essa for a forma escolhida.

## Fontes oficiais

- [CGE-AL — API Transparência](https://www.transparencia.al.gov.br/portal/api)
- [CGE-AL — API de pessoal: lista de servidores ativos](https://www.transparencia.al.gov.br/portal/api/pessoal/servidores-ativos/lista-de-servidores)
- [CGE-AL — API de pessoal: lista de servidores inativos](https://www.transparencia.al.gov.br/portal/api/pessoal/servidores-inativos/lista-de-inativos)
- [CGE-AL — API/exportação CSV, XLS e PDF](https://www.transparencia.al.gov.br/portal/api/exportacao)
- [CGE-AL — Download de Dados](https://www.transparencia.al.gov.br/portal/download-de-dados)
