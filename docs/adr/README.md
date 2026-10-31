# Registros de Decisão de Arquitetura (ADRs)

Cada ADR registra uma decisão arquitetural: o contexto, o que foi decidido e o que
aceitamos como consequência. ADRs aceitos não são reescritos. Quando uma decisão muda,
criamos um novo ADR e marcamos o anterior como substituído.

Para registrar uma decisão, copie [`modelo.md`](modelo.md) com o próximo número.

| ADR | Decisão | Status |
| --- | --- | --- |
| [0001](0001-registrar-decisoes-com-adrs.md) | Registrar decisões com ADRs, em português do Brasil | Aceito |
| [0002](0002-monorepo-em-camadas.md) | Monorepo Cargo em camadas, aplicando SOLID e POO | Aceito |
| [0003](0003-dagron-core-com-executor-proprio.md) | `dagron-core` para o grafo, com executor próprio | Aceito |
| [0004](0004-factory-method-para-fontes.md) | Factory Method para adicionar fontes de dados | Aceito |
| [0005](0005-armazenamento-s3-com-versitygw.md) | API S3 servida pelo versitygw, em vez do LocalStack | Aceito |
| [0006](0006-layout-da-camada-raw.md) | Layout imutável da camada Raw, com manifesto | Aceito |
