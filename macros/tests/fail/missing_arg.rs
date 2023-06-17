fn main() {
    sqlm_macros::sql!("foo{} {}", "bar");
}
