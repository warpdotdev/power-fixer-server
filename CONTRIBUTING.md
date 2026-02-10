# Contributing

## Development setup

1. Install Rust and PostgreSQL.
2. Copy `.env.example` to `.env`.
3. Set required variables (`DATABASE_URL`, `WARP_API_KEY`, `POWERFIXER_ENVIRONMENT_ID`).
4. Run checks:

```bash
cargo fmt --all
cargo check -p power-fixer-server
```
