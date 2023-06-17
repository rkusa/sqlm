fn main() {
    sqlm_macros::sql!("foo{1} {}", "bar");
}
