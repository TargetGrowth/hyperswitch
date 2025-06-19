# Conector Iugu para Hyperswitch

Este documento descreve a integração do conector Iugu na plataforma Hyperswitch.

## Descrição
O conector Iugu permite processar pagamentos, capturas, cancelamentos, sincronizações e reembolsos via API da Iugu.

## Configuração
1. Adicione as credenciais da Iugu no arquivo de configuração (`sample_auth.toml`):

```toml
[iugu]
api_key = "sua_api_key"
```

2. Certifique-se de que o conector está habilitado nas features do projeto.

## Rodando os testes
Execute:
```sh
cargo test --package hyperswitch_connectors --features v1
```

## Exemplos de uso

### Autorização
```rust
// Exemplo ilustrativo
let data = PaymentsAuthorizeData { /* ... */ };
let iugu = Iugu::default();
// let req = iugu.build_authorize_request(&data);
```

### Captura
```rust
// Exemplo ilustrativo
// let req = iugu.build_capture_request(&data);
```

### Cancelamento
```rust
// Exemplo ilustrativo
// let req = iugu.build_cancel_request(&data);
```

### Sincronização
```rust
// Exemplo ilustrativo
// let req = iugu.build_sync_request(&data);
```

### Reembolso
```rust
// Exemplo ilustrativo
// let req = iugu.build_refund_request(&data);
```

## Observações
- Os testes unitários usam mocks e asserts simples para ilustrar o fluxo.
- Para testes de integração reais, configure um ambiente de sandbox na Iugu.

---

Dúvidas ou sugestões? Abra uma issue ou PR! 