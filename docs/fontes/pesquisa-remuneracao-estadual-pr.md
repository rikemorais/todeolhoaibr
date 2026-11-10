# Pesquisa de remuneração estadual do Paraná

**Verificação:** 10/11/2026
**Escopo:** páginas e documentação oficiais do Governo do Paraná; somente interface e informações públicas do portal. Nenhum CSV ou outro arquivo com registros foi baixado nem inspecionado.

## Achados

- O [Portal da Transparência do Paraná — Remuneração do Poder Executivo](https://www.transparencia.pr.gov.br/pte/pessoal/servidores/poderexecutivo/remuneracao) informa que publica remuneração, cargo e local de trabalho de servidores civis e militares, ativos, aposentados, da reserva/reformados, pensionistas, beneficiários de pensões especiais e ex-servidores do Executivo estadual, com histórico desde 2012.
- A mesma página declara exportação em formato aberto CSV para o mês/ano selecionado. A interface oferece seletores de mês e ano e lista anos de 2012 a 2026. O menu “Download do Banco de Dados” também distingue informações funcionais e financeiras, com indicação de arquivo único para informações funcionais. Essa evidência confirma uma exportação interativa mensal, mas não documenta uma URL estável, API, parâmetros HTTP, esquema, codificação, separador ou método de seleção automatizável da competência.
- A [FAQ da Controladoria-Geral do Estado](https://www.cge.pr.gov.br/Pagina/Perguntas-Frequentes), item 18, afirma que as informações do portal podem ser acessadas e usadas livremente, sem autorização ou identificação, inclusive para qualquer finalidade. A FAQ informa que tabelas e bancos são CSV e recomenda UTF-8. Não localizei uma licença formal específica associada à folha nem termos que esclareçam atribuição, redistribuição de derivados ou limites aplicáveis ao conjunto nominal; obter confirmação da CGE antes de publicar derivados.
- A página de remuneração não informa tamanho dos arquivos, limite de requisição, limite de download ou frequência explícita de atualização. A seleção por mês/ano não prova que todas as competências exibidas estejam disponíveis.
- O portal é apresentado pela CGE como portal de transparência do Poder Executivo estadual ([“O que é o Portal”](https://www.cge.pr.gov.br/Pagina/O-que-e-o-Portal)). A consulta não comprova cobertura de Legislativo, Judiciário ou subsídio legal do governador.

## Avaliação

**Prontidão: candidato estruturado, ainda não pronto para ingestão automatizada.** CSV mensal, faixa histórica e orientação UTF-8 estão confirmados oficialmente; a permissão geral de uso também está descrita na FAQ. Faltam contrato técnico reproduzível do download, evidência de qual é a competência mais recente realmente publicada, esquema e tamanhos, além de esclarecimento da licença para redistribuição de derivados.

Antes de implementar, solicitar à CGE ou identificar pela documentação oficial a forma estável de obter o arquivo por competência e as condições específicas de reutilização. Não automatizar chamadas descobertas por inspeção de tráfego sem contrato documentado. Caso o acesso estruturado seja aprovado, manter os arquivos nominais opacos na Raw e não registrar linhas pessoais em logs ou documentação.

## Fontes

- [Portal da Transparência — Remuneração do Executivo](https://www.transparencia.pr.gov.br/pte/pessoal/servidores/poderexecutivo/remuneracao)
- [CGE-PR — Perguntas Frequentes, item 18: dados abertos e download](https://www.cge.pr.gov.br/Pagina/Perguntas-Frequentes)
- [CGE-PR — O que é o Portal](https://www.cge.pr.gov.br/Pagina/O-que-e-o-Portal)
