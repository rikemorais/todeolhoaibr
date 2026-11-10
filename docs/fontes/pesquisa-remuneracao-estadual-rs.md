# Pesquisa de remuneração estadual do Rio Grande do Sul

**Verificação:** 10/11/2026
**Escopo:** documentação e páginas oficiais do Portal da Transparência RS/CAGE. Não acessei consultas de registros, nem baixei ou inspecionei CSVs nominais.

## Achados

- O painel [Pessoal do Poder Executivo (Gov-RS)](https://www.transparencia.rs.gov.br/servidores-aposentados-e-pensionistas/pessoal-do-poder-executivo-gov-rs/dados/) cobre vencimentos de servidores do Executivo estadual, incluindo secretarias, autarquias e fundações, além de servidores inativos e pensionistas. A FAQ oficial cita explicitamente o governador entre os ocupantes contemplados. A página informa que os dados de empresas públicas e sociedades de economia mista sob vínculo CLT são divulgados em páginas próprias, e que outros Poderes têm páginas específicas.
- A [FAQ do Portal](https://www.transparencia.rs.gov.br/faq/) informa que o painel é atualizado mensalmente, que consultas podem ser exportadas como CSV/XLSX e que arquivos históricos estão disponíveis desde janeiro de 2015 em Dados Abertos.
- A área oficial de [Dados Transparência RS](https://www.transparencia.rs.gov.br/dados-abertos/dados-transparencia-rs/dados/) descreve downloads em CSV. A FAQ explica que, ao selecionar um painel, a aba “Download Arquivos” lista os arquivos e a aba “Download Arquivos Layout” lista seus layouts. Arquivos podem ser divididos por mês ou ano devido ao volume; o layout descreve colunas, tamanho, formato e atributos.
- A página de Dados Abertos declara licença aberta permitindo acesso, reutilização e distribuição. A FAQ também caracteriza os dados abertos do portal como utilizáveis e distribuíveis desde que a fonte seja mencionada. Não encontrei nesta documentação o identificador ou a versão de uma licença Creative Commons específica para os CSVs da folha.
- A página de dados abertos renderizada publicamente não lista os recursos em seu conteúdo textual; ela encaminha para um painel cuja lista de arquivos/layouts aparece após a seleção interativa. Por isso, a consulta documental não revelou URL direta estável dos CSVs/layouts, API ou catálogo de metadados que permita enumerar recursos e competências mais recentes sem entrar na interface.
- As páginas consultadas não informam tamanho dos arquivos, limite de download, competência mais recente disponível nem procedimento para determinar essa competência por metadados. A atualização mensal do painel não garante que o arquivo mais recente esteja disponível no mesmo mês.
- A FAQ descreve a remuneração do Executivo com valores brutos e descontos divulgáveis, e esclarece que certos descontos pessoais ou judiciais não são exibidos. Isso não deve ser confundido com remuneração integral ou líquido de contracheque.

## Avaliação

**Prontidão: candidato prioritário para investigação, ainda não pronto para ingestão automatizada.** O RS é relevante para o critério de governadores e servidores e tem documentação mais completa de escopo e atualização mensal do que várias UFs do levantamento. Porém, a documentação pública consultada não fornece URLs estáveis dos arquivos nem metadados suficientes para selecionar com segurança a competência mais recente e estabelecer limites operacionais.

Antes da integração, identificar no próprio catálogo oficial — sem baixar os arquivos nominais — como obter a lista de recursos e layouts e verificar se ela possui endereço estável, datas e tamanhos. Confirmar também o identificador/versão da licença aplicável à folha. Não automatizar downloads por URL inferida da navegação. Se o contrato for confirmado, preservar os CSVs originais opacos na Raw e registrar origem, competência, layout, atribuição e lacunas de escopo.

## Fontes oficiais

- [Portal da Transparência RS — Dados Abertos](https://www.transparencia.rs.gov.br/dados-abertos/)
- [Dados Transparência RS — catálogo/painéis](https://www.transparencia.rs.gov.br/dados-abertos/dados-transparencia-rs/dados/)
- [Portal da Transparência RS — FAQ](https://www.transparencia.rs.gov.br/faq/)
- [Pessoal do Poder Executivo (Gov-RS)](https://www.transparencia.rs.gov.br/servidores-aposentados-e-pensionistas/pessoal-do-poder-executivo-gov-rs/dados/)
