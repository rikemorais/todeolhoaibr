# SIGA Brasil: vinculação de localidade do favorecido ao código IBGE

**Data da pesquisa:** 15/11/2026  
**Pergunta:** é seguro converter `Favorecido do Pagamento - Município/UF` em código municipal IBGE sem credenciais?

## Conclusão

É tecnicamente possível atribuir códigos IBGE a uma parcela dos valores que sejam inequivocamente municípios, mas a documentação e a inspeção disponível não justificam converter todo o campo automaticamente. O campo é uma descrição de favorecido/regionalização do painel, não uma chave municipal; o próprio Senado documenta categorias agregadas e limitações na ligação dos favorecidos. Recomendo manter o texto e a UF originais, e só acrescentar código IBGE quando uma regra determinística validada produzir uma correspondência única dentro da UF ou quando houver alias revisado e versionado. Valores não reconhecidos, agregados e ambíguos devem permanecer sem código. Não usar aproximação fuzzy nem atribuir uma cobertura presumida.

## O que a documentação oficial estabelece

- O [roteiro do Senado para acompanhar pagamentos de emendas](https://www12.senado.leg.br/noticias/materias/2025/12/01/como-monitorar-os-pagamentos-das-emendas-parlamentares) orienta o usuário a consultar a coluna `Favorecido do Pagamento - Município/UF` na tabela de favorecidos. Isso documenta o rótulo e seu propósito de consulta, mas não declara que os valores sejam nomes oficiais de municípios nem que tragam geocódigo IBGE.
- O [Informativo do Painel Favorecido Pagamento do Senado](https://www12.senado.leg.br/orcamento/documentos/outros/siga-paineis/informativo-painel-favorecido-do-pagamento.pdf) diz que o painel apresenta favorecidos da lista da ordem bancária do SIAFI e que o Senado trata as ligações entre favorecido, empenho e classificações. Também informa que, em despesas de folha e benefícios do INSS, os favorecidos são agregados como `Folha de Pagamento` e `Pagamento de benefícios - INSS/FRGPS` (p. 1). Portanto, nem todo favorecido corresponde a uma entidade municipal individual.
- A notícia institucional de lançamento do [Painel Emendas](https://www12.senado.leg.br/noticias/materias/2023/08/23/nova-ferramenta-da-transparencia-a-emendas-parlamentares) descreve regionalização por estados, DF e municípios, além da identificação de CPF/CNPJ ou entidade. Ela não define o campo como código territorial nem publica regra de normalização de nomes.
- O IBGE publica a [Tabela de Códigos de Municípios](https://www.ibge.gov.br/explica/codigos-dos-municipios.php) e informa que o código municipal tem sete dígitos, com os dois primeiros correspondendo à UF; a tabela é atualizada para alterações de criação, fusão e nome de municípios. A [API oficial de Localidades](https://servicodados.ibge.gov.br/api/v1/localidades/municipios) fornece o catálogo de municípios; a [documentação da API](https://servicodados.ibge.gov.br/api/docs/localidades) descreve seus recursos. A UF é, portanto, uma chave de escopo importante, e a versão temporal do catálogo também importa para dados históricos.

## Observações sobre os dados disponíveis

O documento de pesquisa do repositório [`pesquisa-emendas.md`](pesquisa-emendas.md#exportação-verificada-no-painel) registra que, em 15/11/2026, uma exportação pública do painel tinha 781.612 linhas de dados e oito colunas, incluindo município/UF, CPF/CNPJ, natureza, tipo e UF. O mesmo registro afirma que o XLSX não continha código IBGE nem identificador de ordem bancária. Esses são metadados de uma exportação observada, não uma validação da cobertura de correspondência municipal.

Uma primeira tentativa de captura anônima e de consulta à API do IBGE falhou por indisponibilidade de DNS. Depois, aproveitei um snapshot temporário já baixado pelo teste integrado e fiz o perfil local descrito abaixo, sem persistência nos buckets do projeto. Não foi necessário consultar a API do IBGE porque o repositório já mantém um crosswalk versionado com o catálogo e os aliases revisados.

O [JavaScript publicado pelo Senado](https://www9qs.senado.leg.br/extensions/Siga_Brasil_Emendas/Siga_Brasil_Emendas.js), conforme registrado em [`pesquisa-emendas.md`](pesquisa-emendas.md#sessão-pública-e-automação), usa o campo de dimensão “Favorecido do Pagamento - Município/UF” e um campo separado de UF no objeto de favorecidos. A existência da dimensão UF é útil para checagem de consistência, mas não prova por si só que cada nome seja município. O registro local de pesquisa também cita um exemplo de valor no padrão `MUNICIPIO DE BONFIM-BONFIM-RR`.

Uma captura temporária posterior, sem persistência em Raw/Bronze/Silver, foi perfilada localmente e apagada após a contagem. O parser extraiu apenas as colunas de localidade e UF; a tabela de strings do XLSX foi mantida transitoriamente em memória para decodificar essas células. Nenhum valor CPF/CNPJ foi exibido ou salvo separadamente. O arquivo tinha 781.612 registros, 228.967 pares distintos de localidade/UF. Um perfil exploratório por componentes separados por hífen encontrou um único código candidato em 759.112 registros (97,12%), conflitos entre códigos em 10 e nenhum candidato em 22.490. Esse perfil não mediu divergências entre eventual UF no fim do texto e a coluna UF; portanto 97,12% não é a cobertura final publicada. O contrato também consulta a localidade inteira normalizada e rejeita sufixos explícitos de UF divergentes.

## Regra determinística recomendada

1. Preserve sem alteração `Favorecido do Pagamento - Município/UF` e a coluna separada de UF como dados de origem. Não substitua o rótulo pelo nome normalizado.
2. Deixe sem código categorias agregadas, beneficiários não municipais, valores vazios e valores que não possam ser interpretados sem ambiguidade. O contrato bloqueia explicitamente as categorias documentadas de `Folha de Pagamento` e `Pagamento de benefícios - INSS/FRGPS`. A documentação do Senado não fornece uma lista completa de todos os rótulos não municipais, então a cobertura de mapeamento não representa validação semântica de todos os casos.
3. Para candidatos municipais, aplique apenas transformações sintáticas não semânticas previamente especificadas (trim, caixa, normalização Unicode/diacríticos e espaços). O contrato v2 testa a localidade inteira e componentes completos separados por hífen; só remove os prefixos explícitos `MUNICIPIO DE`, `MUNICIPIO DO` e `MUNICIPIO DA` de um componente. Não remove palavras ou sufixos repetidos genericamente.
4. Faça a correspondência por `(UF, nome normalizado)` no catálogo versionado do IBGE. Só aceite resultado quando todos os candidatos encontrados apontarem para um único código nessa UF. Se o rótulo tiver UF explícita no sufixo, ela deve ser igual à coluna UF separada; divergências ficam sem código.
5. Mantenha aliases em tabela explícita com texto de origem, UF, código IBGE, justificativa/fonte e método `alias_manual`. Cada alias precisa apontar para um único código. Não use distância de edição, correspondência aproximada ou município mais próximo como fallback.
6. Reavalie aliases contra a versão temporal apropriada do catálogo, dado que o IBGE documenta mudanças de nomes e composição municipal. Para registros históricos, persista a versão do crosswalk e não reinterprete silenciosamente os códigos em cada execução.

Essa estratégia permite uma associação exata para subconjuntos e torna os casos desconhecidos visíveis. Ainda não é possível afirmar que ela cubra percentual específico do SIGA: isso requer captura de uma exportação atual para armazenamento temporário/memória, perfil completo dos valores distintos e revisão das ambiguidades e divergências de UF antes de qualquer publicação Silver.

## Decisão

**Implementar apenas o enriquecimento parcial determinístico.** O contrato Silver v2 consulta o crosswalk versionado dentro da UF, bloqueia as categorias agregadas documentadas e aceita a localidade inteira ou componentes exatos somente quando todos os candidatos encontrados apontam ao mesmo município e qualquer sufixo de UF coincide com a coluna separada. Não usa distância textual nem escolhe entre municípios conflitantes. `localidade_favorecido` e `uf` permanecem como publicados; `codigo_municipio_ibge`, `nome_municipio_ibge` e `metodo_mapeamento_municipio` ficam nulos para ambiguidades e não correspondências. O perfil de 97,12% mede candidatos, não cobertura final nem precisão manualmente validada; acompanhe mudanças de formato e revise aliases separadamente.

## Fontes primárias

- [Senado: Como monitorar os pagamentos das emendas parlamentares](https://www12.senado.leg.br/noticias/materias/2025/12/01/como-monitorar-os-pagamentos-das-emendas-parlamentares)
- [Senado: Informativo sobre o Painel Favorecido Pagamento (PDF)](https://www12.senado.leg.br/orcamento/documentos/outros/siga-paineis/informativo-painel-favorecido-do-pagamento.pdf)
- [Senado: lançamento do novo Painel Emendas](https://www12.senado.leg.br/noticias/materias/2023/08/23/nova-ferramenta-da-transparencia-a-emendas-parlamentares)
- [Senado: JavaScript público do Painel Emendas](https://www9qs.senado.leg.br/extensions/Siga_Brasil_Emendas/Siga_Brasil_Emendas.js)
- [IBGE: Códigos dos Municípios](https://www.ibge.gov.br/explica/codigos-dos-municipios.php)
- [IBGE: API de Localidades, municípios](https://servicodados.ibge.gov.br/api/v1/localidades/municipios)
- [IBGE: documentação da API de Localidades](https://servicodados.ibge.gov.br/api/docs/localidades)
