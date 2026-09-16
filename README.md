# ViaPost Rust SDK

Official asynchronous Rust SDK for the [ViaPost](https://viapost.io) email API. The beta exposes
typed resources for sending, messages, domains, templates, webhooks, automations and usage.

> Beta `0.1.x`: public names can still change before `1.0.0`. Rust 1.85.1+ is supported.

## Instalação (Português)

O canal principal é uma tag pública no GitHub, sem token:

```toml
[dependencies]
viapost = { git = "https://github.com/ViaPost-io/viapost-rust", tag = "v0.1.1" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

O artefato verificável `viapost-0.1.1.crate` e `SHA256SUMS` também são publicados no
[GitHub Releases](https://github.com/ViaPost-io/viapost-rust/releases). A publicação no crates.io é
opcional e só ocorre por execução manual aprovada.

```rust,no_run
use viapost::{SendRequest, ViaPost};

#[tokio::main]
async fn main() -> Result<(), viapost::Error> {
    let client = ViaPost::new(std::env::var("VIAPOST_API_KEY").expect("VIAPOST_API_KEY"))?;
    let request = SendRequest::new("hello@seu-dominio.com", ["cliente@example.com"])
        .subject("Olá")
        .html("<p>Olá do Rust!</p>")
        .text("Olá do Rust!");

    let result = client.send().create(&request, Some("pedido-123")).await?;
    println!("aceitos: {}", result.accepted.len());
    Ok(())
}
```

Os recursos seguem o mesmo cliente:

```rust,no_run
# async fn example() -> Result<(), viapost::Error> {
# let client = viapost::ViaPost::new("vp_test_example")?;
let messages = client.messages().list(viapost::MessageListParams {
    limit: Some(25),
    ..Default::default()
}).await?;
let detail = client.messages().retrieve("message-id").await?;
let submitted_eml = client.messages().raw("message-id").await?;
let usage = client.usage().retrieve().await?;
# let _ = (messages, detail, submitted_eml, usage);
# Ok(())
# }
```

Webhooks incluem criação e atualização com versão otimista, listagem e inspeção segura de
entregas, evento de teste, replay idempotente e rotação de secret. URLs de destino precisam usar
HTTPS; operações mutáveis que podem ser repetidas exigem uma chave de idempotência explícita.

### Segurança e retries

- timeout global de 60 segundos;
- HTTPS obrigatório, exceto `localhost`, `127.0.0.0/8` e `::1` para desenvolvimento;
- redirects, cookies e proxies de ambiente não são usados;
- somente GET/HEAD repetem `429` e `5xx`, no máximo duas vezes, respeitando `Retry-After`;
- POST/PATCH/DELETE nunca são repetidos automaticamente;
- resposta descomprimida limitada a 8 MiB;
- a API key não aparece em `Debug` nem nos erros do SDK.

## Installation (English)

GitHub tags are the primary, tokenless distribution channel:

```toml
[dependencies]
viapost = { git = "https://github.com/ViaPost-io/viapost-rust", tag = "v0.1.1" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Create a client from a secret environment variable, build a `SendRequest`, and call
`client.send().create(...)` as shown above. Use a stable idempotency key when a business operation
may be submitted more than once. Mutating calls are never retried by the SDK.

## Contract

`openapi.yaml` is vendored from ViaPost `base-code` commit
`891adebbe79a26178fb780ec986172c890a5e261` with SHA-256
`cb61b81b3276679426504eae4161e610eb5520aca2cd71cd267ed62628c518e4`.

## License

[MIT](LICENSE)
