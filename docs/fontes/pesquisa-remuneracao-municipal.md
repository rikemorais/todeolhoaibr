# Pesquisa inicial: remuneração municipal

**Issue:** [#76](https://github.com/rikemorais/todeolhoaibr/issues/76)  
**Data da pesquisa:** 2026-11-10  
**Estado:** triagem de fontes; nenhuma coleta automatizada foi ativada.

## Resumo

O primeiro candidato a agregador é o Sistema de Informações Municipais do
TCE-CE. A documentação oficial da [API de Dados Abertos do SIM](https://api-dados-abertos.tce.ce.gov.br/sim/)
lista recursos específicos de Cadastro de Pessoal e Folha: agentes públicos
municipais, desligamentos, reingressos, itens remuneratórios, folhas de
pagamento e agentes por folha. A página descreve paginação por `$start_index`,
limite de 1.000 registros por requisição e acesso permitido somente a IPs no
Brasil. A documentação também informa que alguns endpoints exigem autorização;
a página de referência não deixa claro, no conteúdo legível, quais rotas de
folha são públicas e quais exigem token.

O TCE-CE publica o [Manual do SIM 2026](https://www.tce.ce.gov.br/municipios/sim/documentacao-e-programas),
atualizado em 19/12/2025, e documentação anual do sistema. Isso indica uma
fonte regulada e com esquema formal, mas ainda não confirma cobertura histórica,
completude das folhas por município, disponibilidade fora do Brasil nem
estabilidade dos identificadores. Antes da ingestão, verificar esses pontos
por chamada de amostra e comparar uma competência com o portal do município.

## Fontes candidatas

| Fonte | Evidência oficial | Avaliação inicial | Pendências |
| --- | --- | --- | --- |
| TCE-CE SIM | [API oficial](https://api-dados-abertos.tce.ce.gov.br/sim/) lista `folhas_pagamentos`, `agentes_publicos_folha` e dimensões de agentes/itens remuneratórios; [manual SIM](https://www.tce.ce.gov.br/municipios/sim/documentacao-e-programas) é publicado por exercício | Melhor primeiro candidato: estrutura agregada estadual, API e paginação documentadas | Confirmar autenticação, endpoint e parâmetros exatos, IP brasileiro, intervalo de competências, município/órgão cobertos e campos pessoais retornados |
| Prefeitura de Porto Alegre | [FAQ oficial](https://transparencia.portoalegre.rs.gov.br/perguntas-frequentes) descreve pesquisa de remuneração por órgão, tipo de folha e competência; afirma atualização mensal até dia 5 | Cobertura funcional útil para comparação independente; parece orientada a consulta web | A página de remuneração usa iframe; não foi localizada nesta triagem uma API ou exportação em lote estável |
| Capitais e demais TCEs | Não avaliados em profundidade nesta etapa | Expansão por adaptador só após confirmar formatos estruturados e licença/termos | Fazer inventário por região e priorizar API ou arquivo oficial reaproveitável |

## Decisão proposta

Fazer uma prova de viabilidade com o TCE-CE antes de implementar um crawler de
portais. A prova deve consultar um município, listar competências disponíveis,
baixar todas as páginas de uma competência e comparar contagens e totais com a
fonte pública municipal. Se a API exigir acesso não disponível ao runtime ou
não tiver cobertura suficiente, avaliar portais de capitais como fontes
independentes, sem presumir que compartilham um esquema.

Não tratar valores exibidos por nome de servidor como chave. A futura ingestão
deve armazenar a resposta original sem mudar identificadores, publicar manifesto
com município, endpoint, competência e instante de coleta, e separar reingestão
de retificações mensais.

## Próxima verificação

1. Inspecionar o OpenAPI do SIM para parâmetros, esquemas, códigos de resposta e
   requisitos de autorização de cada endpoint CPF.
2. Fazer uma chamada pequena de leitura para listar municípios e uma folha
   pública, observando resposta, paginação e bloqueio por localização.
3. Comparar uma competência e seus totais com um portal municipal.
4. Registrar cobertura e limitações antes de escolher o recorte inicial da
   ingestão.
