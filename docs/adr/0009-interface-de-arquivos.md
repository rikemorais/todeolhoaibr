# 0009. Interface web do versitygw para navegar pelos arquivos

- **Status:** Aceito
- **Data:** 2026-10-31

## Contexto

A porta 7070 é a API S3 e só aceita requisições assinadas. Quem abre esse endereço no navegador recebe
`AccessDenied`, e os outros caminhos para ver os dados exigem terminal ou um cliente S3. O painel de execuções
([ADR 0008](0008-painel-de-execucoes.md)) mostra o que aconteceu, mas não o conteúdo gravado.

## Decisão

Ativamos a interface web que já vem no versitygw, publicada em `127.0.0.1:7071`. O login usa as credenciais do `.env`.

A interface roda no navegador e fala diretamente com a API S3, o que exige duas configurações:

- `--webui-gateways` e `--webui-admin-gateways` apontam para `http://localhost:<porta>`. Sem isso, a tela de login sugere
  o IP interno do container, que o navegador não alcança.
- `--cors-allow-origin` libera apenas `http://localhost:<porta da interface>`.

## Consequências

- Dá para navegar e baixar arquivos sem instalar nada e sem escrever código novo.
- **A interface tem poder de administrador:** permite enviar e apagar objetos, o que pode violar a imutabilidade da
  camada Raw ([ADR 0006](0006-layout-da-camada-raw.md)). Aceitamos isso porque o ambiente é local e de uma pessoa, e os dados
  podem ser ingeridos de novo. Isso **precisa ser revisto** antes de qualquer uso compartilhado.
- O acesso precisa ser por `localhost`, porque `127.0.0.1` é outra origem para o CORS.
- A interface é em inglês, o que é uma exceção à [ADR 0001](0001-registrar-decisoes-com-adrs.md), por ser de terceiros.
- A tela mostra um erro 501 no console ao listar usuários: o gateway roda sem o serviço de IAM, e isso não afeta o uso.

## Alternativas consideradas

- **Adicionar um navegador de arquivos ao nosso painel:** seria somente leitura e em português, mas é código a manter para
  algo que já vem pronto. Continua sendo o caminho se a interface deixar de ser adequada.
- **Só documentar `docker compose cp` e clientes S3:** não atende a quem quer ver os dados no navegador.
