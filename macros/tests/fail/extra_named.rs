fn main() {
    sqlm_macros::sql!("{foo}", foo = 1, bar = 2);
}
