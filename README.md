# `sqlm`

An `sql!` macro to write compile-time checked database queries similar to how `format!` works.

[Documentation](https://docs.rs/sqlm-postgres)

## Example

```rust
let id: i64 = 1;
let user: User = sql!("SELECT * FROM users WHERE id = {id}").await?;

#[derive(Debug, FromRow)]
struct User {
    id: i64,
    name: String,
    role: Role,
}

#[derive(Debug, Default, FromSql, ToSql, Enum)]
#[postgres(name = "role")]
enum Role {
    #[default]
    #[postgres(name = "user")]
    User,
    #[postgres(name = "admin")]
    Admin,
}
```

## Usage

- Add `sqlm-postgres` to your dependencies
  ```bash
  cargo add sqlm-postgres
  ```

- Make the `DATABASE_URL` env variable available during compile time (e.g. via adding an `.env` file)
  ```bash
  echo DATABASE_URL=postgres://your-user@localhost/your-db > .env
  ```

- Start using the `sql!` macro (no further setup necessary; a connection pool is automatically created for you)

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
