# Pesquisa de fontes de emendas parlamentares

Pesquisa para a [issue #33](https://github.com/rikemorais/todeolhoaibr/issues/33). Consulta feita em 14/11/2026, usando documentação e catálogos oficiais do Tesouro Nacional, SIOP e CGU.

## Achados

### Tesouro Transparente: repasses mensais

O conjunto oficial [Emendas Parlamentares Individuais e de Bancada](https://www.tesourotransparente.gov.br/ckan/dataset/emendas-parlamentares-individuais-e-de-bancada) fornece um CSV agregado em um arquivo, atualizado mensalmente, com início em janeiro de 2015. A página lista um CSV de 67,1 MiB e licença ODbL. O recurso atual está descrito no [CKAN](https://www.tesourotransparente.gov.br/ckan/dataset/emendas-parlamentares-individuais-e-de-bancada/resource/66d69917-a5d8-4500-b4b2-ef1f5d062430); a URL de download e a URL da API CKAN `package_show` estão na página.

O [metadado oficial em PDF](https://www.tesourotransparente.gov.br/ckan/dataset/83e419da-1552-46bf-bfc3-05160b2c46c9/resource/cb87c4ee-7c3e-4e38-89ef-6413bc4cc45b/download/Metadados-Emendas.pdf) define a cobertura como transferências de emendas individuais e de bancada para estados, municípios e DF, em competência mensal e regime de caixa. A inspeção do CSV oficial atual confirmou 15 colunas: nome do ente, UF, código SIAFI, código IBGE, data, ano, mês, tipo de ente, ordem bancária (OB), CNPJ e nome do favorecido, nome/tipo da emenda, transferência especial, categoria econômica e valor. O arquivo é Latin-1, delimitado por `;`.

**Limite para o objetivo da issue:** esse CSV contém código IBGE municipal e permite acompanhar repasses por OB e favorecido, mas não identifica o autor parlamentar. A OB é um identificador candidato de transferência; antes de tratá-la como chave global, é necessário verificar colisões e estabilidade nos dados reais. O manifesto Raw e a idempotência por URL/versão do arquivo continuam aplicáveis mesmo se houver repetição de linhas no conteúdo.

### SIOP: autoria e beneficiários indicados

O [catálogo oficial do Conecta gov.br para Emendas Individuais SIOP](https://www.gov.br/conecta/catalogo/apis/emendas-individuais-siop) lista operação de obtenção de emendas e descreve consulta de emendas individuais, autores e beneficiários. A documentação técnica disponibilizada pelo próprio SIOP é o [manual da API do Orçamento Impositivo](https://www1.siop.planejamento.gov.br/siopdoc/doku.php/api:impositivo); ele mostra uma requisição autenticada para `POST https://www.siop.planejamento.gov.br/modulo/impositivo/itens/api`, com exercício e autor, retornando classificações, valores e detalhamento dos beneficiários.

Para consulta mais ampla, o manual oficial [Consulta de Emendas e Autores via Web Service](https://www1.siop.planejamento.gov.br/siopdoc/doku.php/consulta_de_emendas_e_autores_via_web_service) descreve `WSAlteracoesOrcamentarias`, as operações `obterEmendasAprovadas` e `obterAutores`, filtros por exercício, órgão/unidade, código de parlamentar e tipo de resultado primário, além dos campos de autoria e identificador de localizador. A resposta inclui identificadores como `identificadorUnicoLocalizador`, código/número de emenda, código e nome do parlamentar, partido, UF e valor. O exemplo da emenda também possui `beneficiariosEmenda`; a documentação consultada não detalha nessa estrutura um código IBGE municipal estável.

Há dois mecanismos SIOP distintos. A API GraphQL do módulo Orçamento Impositivo usa as permissões de um usuário SIOP cadastrado; não exige credencial especial de Web Service nem formulário. O usuário obtém um token Bearer pelo endpoint de login, válido por 48 horas, e consulta a API de teste em `https://testews.siop.gov.br/modulo/impositivo/itens/api` antes da produção. A documentação atualizada do schema está no Explorador de API e também requer usuário com as permissões correspondentes. Isso continua sendo acesso autenticado e depende de uma conta SIOP; portanto, a integração não pode ser executada sem essa conta/token.

O manual separado [Consulta de Emendas e Autores via Web Service](https://www1.siop.planejamento.gov.br/siopdoc/doku.php/consulta_de_emendas_e_autores_via_web_service) descreve o caminho SOAP `WSAlteracoesOrcamentarias`, com operações `obterEmendasAprovadas` e `obterAutores`, filtros por exercício, órgão/unidade, código de parlamentar e tipo de resultado primário. Esse caminho usa credenciais e perfil de Web Service; não se deve confundi-lo com a API GraphQL. A documentação pública não detalha um código IBGE municipal estável em `beneficiariosEmenda`. Não reutilizar senhas em texto claro nem incluir CPF do parlamentar sem necessidade.

**Implicação:** SIOP tem campos de autor e beneficiários, mas não há evidência suficiente na documentação consultada para prometer que cada beneficiário equivale a município/código IBGE ou que cada registro represente dinheiro efetivamente transferido. É preciso inspecionar a resposta real de um exercício e comparar com a semântica do conjunto Tesouro antes de juntar os dois. O identificador único do localizador identifica a programação/localizador, não necessariamente uma ordem bancária ou uma linha de beneficiário.

### Portal da Transparência (CGU)

A [documentação oficial Swagger da API do Portal da Transparência](https://api.portaldatransparencia.gov.br/swagger-ui/index.html) publica `GET /api-de-dados/emendas` (“Consulta as emendas parlamentares”) e `GET /api-de-dados/emendas/documentos/{codigo}` (documentos por código de emenda). O [catálogo de APIs do Conecta gov.br](https://www.gov.br/conecta/catalogo/apis/portal-da-transparencia-do-governo-federal) confirma que a API publica dados sobre emendas e aponta para o Swagger de produção.

O endpoint é uma alternativa para enriquecer a série: o schema `ConsultaEmendasDTO` da especificação OpenAPI lista `codigoEmenda`, `ano`, `tipoEmenda`, `autor`, `nomeAutor`, `numeroEmenda`, `localidadeDoGasto` e valores empenhado, liquidado, pago e restos a pagar. A especificação não define `localidadeDoGasto` como código municipal IBGE; tratá-lo como texto, sem inferir código. O endpoint exige a chave no cabeçalho `chave-api-dados`, obtida no [cadastro da API](https://portaldatransparencia.gov.br/api-de-dados/cadastrar-email). Os parâmetros documentados incluem ano e página, mas não UF.

## Integração e limites restantes

1. A fonte `emendas` ingere o CSV completo do Tesouro em `repasses/tesouro` e as respostas da API CGU em recursos anuais `cgu/ano=AAAA`; ambas preservam conteúdo e manifesto na Raw.
2. A consulta CGU usa o filtro `ano`, páginas até array vazio, o token `PORTAL_TRANSPARENCIA_TOKEN` e o limite de 90 requisições por minuto existente no cliente.
3. A inspeção do CSV confirmou UF e código IBGE, além de ordem bancária e favorecido. A API CGU fornece autoria, localidade como texto e valores. Os grãos não são unidos nem deduplicados na ingestão.
4. A API SIOP não foi integrada: a API GraphQL requer usuário SIOP com as permissões adequadas e token Bearer temporário; o Web Service SOAP é um mecanismo separado com credencial/perfil próprios. A estrutura de beneficiários ainda precisa de inspeção de amostra.
5. A Silver oferece contratos separados para CGU (`emendas-cgu`) e Tesouro (`emendas-tesouro`), sem cruzamento. Os filtros Silver de ano valem para ambos; UF só vale para o Tesouro. As saídas filtradas usam recursos próprios e preservam a publicação integral. Veja [contratos e comandos](emendas.md#refino-silver).
6. A Issue #33 não está totalmente atendida: o Tesouro não identifica o autor parlamentar, a CGU não fornece código IBGE/UF estruturada e a SIOP ainda não foi integrada. Sem acesso SIOP, permanecem pendentes a integração GraphQL e a inspeção de uma resposta real de beneficiários. A validação de colisões/estabilidade da OB pode avançar independentemente de SIOP usando o CSV público do Tesouro. Nenhum relacionamento autor-pagamento ou município+ano é presumido.

## Fontes primárias

- [Dataset Tesouro: Emendas Parlamentares Individuais e de Bancada](https://www.tesourotransparente.gov.br/ckan/dataset/emendas-parlamentares-individuais-e-de-bancada)
- [Metadados do dataset do Tesouro (PDF)](https://www.tesourotransparente.gov.br/ckan/dataset/83e419da-1552-46bf-bfc3-05160b2c46c9/resource/cb87c4ee-7c3e-4e38-89ef-6413bc4cc45b/download/Metadados-Emendas.pdf)
- [API de Emendas Individuais SIOP no Conecta gov.br](https://www.gov.br/conecta/catalogo/apis/emendas-individuais-siop)
- [Manual SIOP: API Orçamento Impositivo](https://www1.siop.planejamento.gov.br/siopdoc/doku.php/api:impositivo)
- [Manual SIOP: orientações de acesso à API](https://www1.siop.planejamento.gov.br/siopdoc/doku.php/orientacoes_de_acesso)
- [Manual SIOP: consulta de emendas e autores via Web Service](https://www1.siop.planejamento.gov.br/siopdoc/doku.php/consulta_de_emendas_e_autores_via_web_service)
- [Swagger da API do Portal da Transparência (CGU)](https://api.portaldatransparencia.gov.br/swagger-ui/index.html)
- [API do Portal da Transparência no Conecta gov.br](https://www.gov.br/conecta/catalogo/apis/portal-da-transparencia-do-governo-federal)
