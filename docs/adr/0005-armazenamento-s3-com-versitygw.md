# 0005. API S3 servida pelo versitygw, em vez do LocalStack

- **Status:** Aceito
- **Data:** 2026-10-31

## Contexto

A camada Raw precisa de armazenamento local que qualquer pessoa consiga subir, sem conta em nenhum
serviço, inclusive em computadores modestos. Os dados precisam sobreviver ao reinício dos containers,
já que a Raw é a origem de todas as etapas futuras.

O LocalStack era a primeira ideia. Em março de 2026 ele passou a exigir conta e token de autenticação
nas versões atuais, e deixou de publicar a imagem Community.

Comparação feita em 2026-10-31, em um Mac ARM com Docker Desktop:

| Critério | LocalStack `latest` | LocalStack `4.14` | versitygw `v1.10.0` |
| --- | --- | --- | --- |
| Roda sem conta | Não (encerra com código 55) | Sim | Sim |
| Tamanho da imagem | 1,68 GB | 1,8 GB | 108 MB |
| Memória em repouso | Não medido | 142 MiB | 16 MiB |
| Dados após reiniciar | Não medido | **Perdidos** | Mantidos |

O teste com o `4.14` usou `SERVICES=s3`, `PERSISTENCE=1` e volume montado. Depois de reiniciar o
container, o bucket deixou de existir (`NoSuchBucket`).

## Decisão

- O domínio conhece apenas a porta `ArmazenamentoRaw`, e sua implementação usa a API S3.
- Localmente, a API S3 é servida pelo [versitygw](https://github.com/versity/versitygw), com o
  backend POSIX em um volume Docker e a versão da imagem fixada.
- Usamos a região `sa-east-1`. Com `us-east-1`, o `rust-s3` envia um `LocationConstraint` que o padrão
  S3 rejeita (`InvalidLocationConstraint`), porque endpoints personalizados não entram na exceção que
  o crate faz para essa região.
- A porta é publicada apenas em `127.0.0.1`. As credenciais padrão protegem só o ambiente local.

## Consequências

- Nenhuma conta nem token é necessário, e o consumo cabe em máquinas modestas.
- Levar a Raw para um S3 real ou outro compatível é questão de configuração, sem alterar código.
- Os objetos são arquivos comuns no volume, fáceis de inspecionar ou copiar.
- O versitygw é um gateway, não um emulador da AWS. Se o projeto precisar de outros serviços, como SQS
  ou Lambda, a escolha deve ser reavaliada.
- `docker compose down -v` apaga o volume e, com ele, todos os dados da Raw.

## Alternativas consideradas

- **LocalStack atual:** exige conta de cada pessoa, o que contraria o objetivo de rodar sem barreiras.
- **LocalStack `4.14` fixado:** não persiste os dados e não recebe mais atualizações.
- **MinIO:** o repositório `minio/minio` no Docker Hub não listou nenhuma tag recente na consulta feita.
- **Garage:** imagem ainda menor (27 MB), mas exige configurar o layout do cluster e criar as chaves
  antes do primeiro uso, o que complica o início.
- **RustFS e SeaweedFS:** oferecem mais recursos do que precisamos. Não foram testados; a imagem do RustFS
  tem 115 MB.
- **Sistema de arquivos direto:** mais simples hoje, mas perde a compatibilidade com S3, útil para as
  ferramentas de transformação e para uma futura migração à nuvem.
