# Pesquisa de remuneração estadual de São Paulo

**Verificação:** 10/11/2026
**Escopo:** páginas oficiais do Portal da Transparência do Estado de São Paulo e do catálogo Dados Abertos SP. Não baixei nem abri arquivos de remuneração, não consultei endpoints que retornem linhas de servidores e não registrei dados nominais.

## Achados

- O [Portal da Transparência — Relação nominal e remuneração mensal](https://www.transparencia.sp.gov.br/Remuneracaomensal/Index) identifica o escopo como Poder Executivo do Estado de São Paulo. A página oferece exportação CSV/PDF da consulta, um dicionário de dados e uma seção “Download de Arquivos de Remuneração” com opções TXT e CSV para servidores ativos, aposentados, pensionistas e todos os servidores estaduais. Também lista uma série histórica mensal em ZIP. A página consultada exibe competências de janeiro de 2023 a agosto de 2026; isso comprova a listagem visível no momento da verificação, não que cada arquivo esteja íntegro ou acessível.
- A listagem do portal é renderizada dinamicamente: a versão acessível sem JavaScript mostra as categorias e competências, mas não revela os endereços diretos dos arquivos. Portanto, não foi possível confirmar uma URL estável de download por competência, nem um padrão de nomes ou parâmetros que possa ser usado como contrato automatizado. As opções de exportação da consulta também não documentam uma URL ou interface de integração.
- O [catálogo Dados Abertos SP — Informações Sobre os Servidores](https://dadosabertos.sp.gov.br/dataset/informacoes-sobre-os-servidores) apresenta um recurso descrito como “Remuneração Mensal” e rotulado CSV, mas o link do recurso remete ao portal de transparência, não a um arquivo CSV direto. O catálogo registra última atualização em 17/04/2025 e não informa competência, tamanho do arquivo nem `last modified` do recurso. Assim, o catálogo não serve hoje como índice atualizado de arquivos mensais.
- O catálogo indica “Nenhuma Licença Fornecida” para esse conjunto. O portal de transparência, por sua vez, exibe no rodapé “Licença Creative Commons 4.0”, mas não identifica a variante da licença nem esclarece explicitamente se ela se aplica aos arquivos de remuneração e a seus derivados. Não considerar isso autorização inequívoca de redistribuição; é necessário obter esclarecimento oficial antes de publicar derivados.
- Há outra página oficial de relatório de remuneração, [PortalTransparencia-Report/Remuneracao.aspx](https://www.transparencia.sp.gov.br/PortalTransparencia-Report/Remuneracao.aspx), que exibe uma série histórica em formato de seleção e, na captura consultada, vai até março de 2026. A diferença para a página principal, que lista até agosto de 2026, sugere versões ou atualizações distintas do portal. Não usei essa discrepância para inferir disponibilidade real dos arquivos nem consultei os relatórios.
- A [seção Pessoal do Portal](https://www.transparencia.sp.gov.br/Home/Servidor) confirma que a relação nominal e a remuneração mensal são informações do quadro funcional do Executivo estadual. A FAQ do portal informa que a periodicidade de atualização depende da informação (pode ser diária, mensal ou anual), sem especificar um calendário para os arquivos de remuneração.

## Avaliação

**Prontidão: há arquivos públicos anunciados, mas falta um contrato verificável para ingestão automatizada.** O portal confirma as categorias de arquivos TXT/CSV e uma série histórica mensal compactada, mas não expõe na página consultável URLs diretas estáveis, tamanho, data de atualização por arquivo ou metadados legíveis por máquina sobre competências. O catálogo tem um recurso apontando para a página do portal, está desatualizado em relação à série exibida e não fornece licença para o conjunto.

Antes de implementar uma coleta, confirmar oficialmente com a Controladoria Geral do Estado (gestora indicada no portal) o endereço e o padrão de download, a competência mais recente efetivamente publicada, a periodicidade de atualização, os metadados de tamanho e a licença aplicável aos arquivos e derivados. Não inferir endpoints internos a partir do funcionamento da interface. Caso a integração seja aprovada, tratar os arquivos como dados pessoais e evitar conteúdo nominal em documentação, logs e artefatos de teste.

## Fontes oficiais

- [Portal da Transparência — Relação nominal e remuneração mensal](https://www.transparencia.sp.gov.br/Remuneracaomensal/Index)
- [Portal da Transparência — Relatório de remuneração e série histórica](https://www.transparencia.sp.gov.br/PortalTransparencia-Report/Remuneracao.aspx)
- [Dados Abertos SP — Informações Sobre os Servidores](https://dadosabertos.sp.gov.br/dataset/informacoes-sobre-os-servidores)
- [Portal da Transparência — Pessoal](https://www.transparencia.sp.gov.br/Home/Servidor)
- [Portal da Transparência — Perguntas Frequentes](https://www.transparencia.sp.gov.br/Home/Perguntas)
