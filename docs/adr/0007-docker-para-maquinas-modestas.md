# 0007. Docker como única dependência, pensando em máquinas modestas

- **Status:** Aceito
- **Data:** 2026-10-31

## Contexto

Qualquer pessoa deve conseguir rodar o projeto, mesmo sem um computador potente. Compilar Rust é a parte
mais pesada, e exigir a instalação da linguagem afastaria quem só quer executar a ingestão.

## Decisão

Docker com Compose é o único requisito para rodar o projeto.

- **Execução sob demanda:** o serviço `ingestao` existe apenas enquanto um comando roda. Em repouso, só
  o armazenamento fica no ar.
- **Limites de memória:** 128 MB para o armazenamento e 256 MB para a ingestão.
- **Imagem final mínima:** distroless, sem root e sem compilador, com apenas o binário (56 MB).
- **Compilação controlada:** build em etapas, com cache de dependências via BuildKit e número de jobs
  configurável (`TODEOLHO_BUILD_JOBS`, 2 por padrão).
- **Perfil `release` para compilar com menos recursos:** sem LTO, `codegen-units = 16` e `opt-level = "s"`.
  A ingestão é dominada por espera de rede, então o desempenho máximo do binário importa pouco.
- **Dependências leves:** `rust-s3` e `rustls` com `ring`, em vez do `aws-sdk-s3`, OpenSSL ou `aws-lc`, que
  exigem mais tempo de compilação e ferramentas de C adicionais.
- **Gentileza com as APIs:** concorrência baixa e espera crescente entre retentativas.

## Consequências

- Em um Mac ARM com 6 CPUs, a primeira compilação levou 148 segundos usando 2 jobs. Ela ainda não foi medida
  em uma máquina realmente modesta, e esse é o ponto mais pesado da experiência.
- Quando o repositório estiver no GitHub, publicar a imagem pronta em um registro eliminará a compilação
  para quem só quer executar a ingestão.
- Os testes unitários e o desenvolvimento diário continuam exigindo Rust instalado ou um container próprio.
- Os limites de memória podem precisar de ajuste quando surgirem fontes com respostas grandes.

## Alternativas consideradas

- **Instalar Rust localmente:** mais rápido para quem desenvolve, mas barra quem só quer executar.
- **`aws-sdk-s3`:** é o SDK oficial, porém compila muito mais código do que precisamos.
- **Imagem final `scratch` ou musl:** um pouco menor, mas exige mais cuidado com certificados e linkagem. A
  distroless resolve isso com poucos megabytes a mais.
